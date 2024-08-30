use crate::database::ItemId;

#[derive(Clone)]
pub struct UnresolvedIdent {
    pub parts: Vec<String>,
}

impl std::fmt::Debug for UnresolvedIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnresolvedIdent({}", &self.parts[0])?;

        for p in &self.parts[1..] {
            write!(f, ".{}", p)?;
        }

        write!(f, ")",)
    }
}

#[derive(Debug)]
pub enum UnresolvedAST {
    Call { ident: UnresolvedIdent },
}

#[derive(Debug)]
pub enum ResolvedAST {
    Call { ident: ItemId },
}
