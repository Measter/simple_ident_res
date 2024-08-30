use std::collections::BTreeMap;

use crate::ast::{ResolvedAST, UnresolvedAST, UnresolvedIdent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Function,
    Module,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemId(usize);

impl std::fmt::Debug for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemId({})", self.0)
    }
}

#[derive(Debug)]
pub struct ItemHeader {
    kind: ItemKind,
    name: String,
    parent: ItemId,
    id: ItemId,
}

pub struct Scope {
    unresolved_imports: Vec<UnresolvedIdent>,
    children: BTreeMap<String, ItemId>,
}

impl Scope {
    fn new() -> Self {
        Self {
            unresolved_imports: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    fn add_child(&mut self, name: String, id: ItemId) {
        // Might want to handle name collision here.
        self.children.insert(name, id);
    }
}

pub struct Database {
    headers: Vec<ItemHeader>,
    root: ItemId,
    // BTreeMap is just so the printing has the same order each time.
    unresolved_bodies: BTreeMap<ItemId, Vec<UnresolvedAST>>,
    resolved_bodies: BTreeMap<ItemId, Vec<ResolvedAST>>,
    scopes: Vec<Scope>,
}

impl Database {
    pub fn new() -> Self {
        let mut s = Self {
            headers: Vec::new(),
            root: ItemId(0),
            unresolved_bodies: BTreeMap::new(),
            resolved_bodies: BTreeMap::new(),
            scopes: Vec::new(),
        };

        s.new_item("<ROOT>".to_owned(), ItemKind::Module, None);

        s
    }

    pub fn new_item(&mut self, name: String, kind: ItemKind, parent: Option<ItemId>) -> ItemId {
        let id = ItemId(self.headers.len());
        let parent = parent.unwrap_or(self.root);

        self.headers.push(ItemHeader {
            kind,
            name: name.clone(),
            parent,
            id,
        });

        self.scopes.push(Scope::new());
        self.scopes[parent.0].add_child(name, id);

        id
    }

    fn get_header(&self, item_id: ItemId) -> &ItemHeader {
        &self.headers[item_id.0]
    }

    pub fn set_unresolved_body(&mut self, id: ItemId, body: Vec<UnresolvedAST>) {
        self.unresolved_bodies.insert(id, body);
    }

    pub fn get_unresolved_body(&self, id: ItemId) -> &[UnresolvedAST] {
        &self.unresolved_bodies[&id]
    }

    pub fn set_resolved_body(&mut self, id: ItemId, body: Vec<ResolvedAST>) {
        self.resolved_bodies.insert(id, body);
    }

    fn get_scope(&self, id: ItemId) -> &Scope {
        &self.scopes[id.0]
    }

    pub fn add_import(&mut self, id: ItemId, ident: UnresolvedIdent) {
        self.scopes[id.0].unresolved_imports.push(ident);
    }

    pub fn resolve_idents(&mut self) {
        // The first thing we do is resolve idents on the scopes. This is because resolution of item bodies
        // will look at it's parent module's scope for symbols.
        let item_ids: Vec<_> = self.headers.iter().map(|h| h.id).collect();

        for &item_id in &item_ids {
            let imports = self.get_scope(item_id).unresolved_imports.clone();

            for import in imports {
                let name = import.parts.last().unwrap().clone();
                let resolved_id = self.resolve_single_ident(item_id, &import);

                self.scopes[item_id.0].add_child(name, resolved_id);
            }
        }

        // Now we iterate over the function bodies, and resolve idents within those.
        for item_id in item_ids {
            if self.get_header(item_id).kind != ItemKind::Function {
                continue;
            }

            let body = self.get_unresolved_body(item_id);
            let new_body = self.resolve_idents_in_body(item_id, body);
            self.set_resolved_body(item_id, new_body);
        }
    }

    fn resolve_idents_in_body(
        &self,
        current_func: ItemId,
        body: &[UnresolvedAST],
    ) -> Vec<ResolvedAST> {
        let mut new_body = Vec::new();

        for node in body {
            match node {
                UnresolvedAST::Call { ident } => {
                    let resolved_ident = self.resolve_single_ident(current_func, ident);
                    new_body.push(ResolvedAST::Call {
                        ident: resolved_ident,
                    });
                }
            }
        }

        new_body
    }

    fn resolve_single_ident(&self, item_id: ItemId, ident: &UnresolvedIdent) -> ItemId {
        // The first part of the ident (e.g. "A2" in "A2.a_func") is where we start traversing *down*
        // into the module tree.

        // But first, we need to find out what item the first part refers to. To do that we need to
        // traverse *up* the module tree, starting from the current item, looking for a matching ID.
        // The current item here would be, for example, a function that we're resolving the body for.
        // This would be where you would plug in something like Rust's "crate", "super" or "self" path
        // segments.
        let root = self.get_visible_symbol(item_id, &ident.parts[0]);

        // Now that we know what the root is, we can start traversing down the tree into its children.
        let mut current_item = root;
        for sub_ident in &ident.parts[1..] {
            let current_header = self.get_header(current_item);
            if current_header.kind != ItemKind::Module {
                panic!("Cannot resolve into non-modules {}", sub_ident);
            }

            let cur_scope = self.get_scope(current_item);
            let child_id = *cur_scope.children.get(sub_ident).unwrap();

            current_item = child_id;
        }

        // Once we've got through the sub-idents, we're done.
        current_item
    }

    fn get_visible_symbol(&self, item_id: ItemId, name: &str) -> ItemId {
        // First, we check ourselves. It's valid for an item to refer to itself, so that should
        // come first.
        let own_header = self.get_header(item_id);
        if name == own_header.name {
            return item_id;
        }

        // Now we check our children.
        let own_scope = self.get_scope(item_id);
        if let Some(child_id) = own_scope.children.get(name) {
            return *child_id;
        }

        // If we are not a module, we then check out parent module's children.
        // The reason we don't traverse up if we're a module, or traverse upward
        // past our parent module is so that we only see symbols imported into
        // *our* module.
        if own_header.kind != ItemKind::Module {
            // In this, we don't allow nested functions, so a function's parent is known
            // to be a module. If you do allow them, then you may want to repeat this logic
            // in each scope until you get to a module.
            let parent_scope = self.get_scope(own_header.parent);
            if let Some(child) = parent_scope.children.get(name) {
                return *child;
            }
        }

        // If we still haven't found a symbol, we check the root.
        // In the example file, the roots would be A1 and B1.
        let root_scope = self.get_scope(self.root);
        if let Some(child) = root_scope.children.get(name) {
            return *child;
        }

        panic!("symbol not found");
    }

    pub fn print_headers(&self) {
        eprintln!(" == Headers ==");
        eprintln!("{:#?}\n\n", self.headers);
    }

    pub fn print_unresolved_ast(&self) {
        eprintln!(" == Unresolved ASTs ==");
        eprintln!("{:#?}\n\n", self.unresolved_bodies);
    }

    pub fn print_resolved_ast(&self) {
        eprintln!(" == Resolved ASTs ==");
        eprintln!("{:#?}", self.resolved_bodies);
    }
}
