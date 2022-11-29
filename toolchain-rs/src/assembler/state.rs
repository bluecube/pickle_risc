use id_arena::Arena;
use std::collections::HashMap;

use crate::instruction::Word;

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
    pub(super) current_pc: Word,
}

impl ParseState {
    /// Initialize the ParseState at the beginning of first pass
    pub fn new() -> ParseState {
        Default::default()
    }

    pub fn start_second_pass(&mut self) {
        self.first_pass = false;
    }

    /// Define a symbol with the given value or check that it is already defined with identical value
    pub(super) fn define_symbol(
        &mut self,
        sym_name: &str,
        value: Value,
        sectioned: bool,
        scope_id: Option<ScopeId>,
    ) -> Result<(), String> {
        if sym_name.find(SCOPE_PATH_SEP).is_some() {
            Err("Symbol definition can't contain path separators")?;
        }
        let active_scope = &mut self.scopes[*self.active_scopes.last().unwrap()].0;
        let previous_def = active_scope.get(sym_name);
        let current_section = if sectioned { Some(self.current_section) } else { None };
        if self.first_pass {
            if let Some(_) = previous_def {
                Err(format!("Redefinition of symbol {:?}", sym_name))?;
            } else {
                active_scope.insert(
                    sym_name.into(),
                    Symbol {
                        value,
                        section: current_section,
                        scope: scope_id,
                    },
                );
            }
        } else {
            if let Some(prev) = previous_def {
                if (prev.value != value)
                    | (prev.section != current_section)
                    | (prev.scope != scope_id)
                {
                    Err(format!(
                        "Symbol {:?} changed value between passes",
                        sym_name
                    ))?;
                }
            } else {
                Err(format!(
                    "Symbol {:?} was only defined in second pass! (assembler error?)",
                    sym_name
                ))?;
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

#[derive(Clone, Debug)]
struct Symbol {
    /// Value of the symbol (typically offset in a section)
    value: Value,
    /// Index into section arena, specifying the section where this symbol is defined
    section: SectionId,
    /// Index into scope arena, if there is a scope attached to this symbol
    scope: Option<ScopeId>, // Each symbol may have an attached scope
}

#[derive(Clone, Debug, Default)]
pub(super) struct Section {
    start_address: Word,
    pc: Word,
}
