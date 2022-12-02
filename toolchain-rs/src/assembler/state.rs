use id_arena::Arena;
use std::collections::HashMap;

use crate::instruction::Word;
use crate::assembler::{
    parser::{ParseError, ParserResult},
    lexer::Span,
};

pub(super) type ScopeId = id_arena::Id<Scope>;
pub(super) type SectionId = id_arena::Id<Section>;

const SCOPE_PATH_SEP: char = ':';

pub(super) type Value = i32;

#[derive(Clone, Debug)]
pub struct ParseState {
    first_pass: bool,

    scopes: Arena<Scope>,
    /// Stack of active scopes, always contains at least one item.
    active_scopes: Vec<ScopeId>,

    sections: Arena<Section>,
    section_names: HashMap<String, SectionId>,
    current_section: SectionId,
    current_pc: Word,
}

impl ParseState {
    /// Initialize the ParseState at the beginning of first pass
    pub fn new() -> ParseState {
        Default::default()
    }

    pub fn start_second_pass(&mut self) {
        self.first_pass = false;
    }

    pub (super) fn current_pc_symbol(&self, attached_scope: Option<ScopeId>, defined_at: Span) -> Symbol {
        Symbol::Location {
            section: self.current_section,
            offset: self.current_pc,
            attached_scope,
            defined_at
        }
    }

    /// Define a symbol with the given value or check that it is already defined with identical value
    pub(super) fn define_symbol(
        &mut self,
        sym_name: &str,
        symbol: Symbol,
    ) -> ParserResult<()> {
        if sym_name.find(SCOPE_PATH_SEP).is_some() {
            return Err(ParseError::OtherError { description: "Symbol definition can't contain path separators".to_owned() });
        }
        let active_scope = &mut self.scopes[*self.active_scopes.last().unwrap()].0;
        let previous_def = active_scope.get(sym_name);
        if self.first_pass {
            if let Some(previous_def) = previous_def {
                return Err(ParseError::SymbolRedefinition { span: symbol.get_defined_at(), previous_definition: previous_def.get_defined_at() });
            } else {
                active_scope.insert(
                    sym_name.into(),
                    symbol
                );
            }
        } else {
            if previous_def != Some(&symbol) {
                Err(ParseError::SymbolChangedValue { span: symbol.get_defined_at() })?;
            }
        }

        Ok(())
    }

    pub(super) fn get_symbol_value(&self, symbol: &str) -> Option<Value> {
        /*if let Some(rest) = symbol.strip_prefix(SCOPE_PATH_SEP) {
            assert_eq!(self.active_scopes.first().unwrap(), ROOT_SCOPE_INDEX);
            self.scopes_arena[ROOT_SCOPE_INDEX].lookup_symbol_recursive(rest)
        } else {
            self.active_scopes.iter().rev().find_map(|scope_id| self.scopes_arena[*scope_id].lookup_symbol_recursive(symbol))
        }*/
        todo!();
    }

    pub(super) fn push_scope<'a>(&mut self) -> ScopeId {
        todo!();
    }

    pub(super) fn pop_scope<'a>(&mut self) {
        todo!();
    }
}

impl Default for ParseState {
    fn default() -> ParseState {
        let mut scopes = Arena::new();
        let root_scope = scopes.alloc(Scope(HashMap::new()));
        let mut sections = Arena::new();
        let text_section = sections.alloc(Section {
            start_address: 0,
            pc: 0,
        });

        ParseState {
            first_pass: true,
            scopes,
            active_scopes: vec![root_scope],
            sections,
            section_names: HashMap::from([(".text".into(), text_section)]),
            current_section: text_section,
            current_pc: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Scope(HashMap<String, Symbol>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Symbol {
    /// Symbol that represents a location in the code. It belongs to a section and
    /// might have a scope attached.
    Location {
        section: SectionId,
        offset: Word,
        attached_scope: Option<ScopeId>,
        defined_at: Span,
    },
    /// Symbol that is just a value, not attached to anything
    Free {
        value: Value,
        defined_at: Span,
    },
}

impl Symbol {
    fn get_value(&self, state: &ParseState) -> Value {
        match self {
            Symbol::Location { section, offset, attached_scope: _, defined_at: _ } =>
                Value::from(state.sections[*section].start_address) + Value::from(*offset),
            Symbol::Free { value, defined_at: _ } => *value,
        }
    }

    fn get_defined_at(&self) -> Span {
        match self {
            Symbol::Location { section: _, offset: _, attached_scope: _, defined_at } => defined_at.clone(),
            Symbol::Free { value: _, defined_at } => defined_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Section {
    start_address: Word,
    pc: Word,
}
