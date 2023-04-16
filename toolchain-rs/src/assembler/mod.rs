pub mod expr_parser;
pub mod files;
pub mod lexer;
pub mod parser;

use codespan_reporting::diagnostic::Diagnostic;
use id_arena::Arena;
#[cfg(test)]
use mockall::automock;
use std::borrow::Cow;
use std::collections::HashMap;
use thiserror::Error;

use crate::assembler::files::{FileId, InputFiles, Location};
use crate::assembler::parser::parse_file;
use crate::instruction::Word;

pub(super) type ScopeId = id_arena::Id<Scope>;
pub(super) type SectionId = id_arena::Id<Section>;

const SCOPE_PATH_SEP: char = ':';

pub(super) type Value = i32;

#[derive(Clone, Debug)]
pub struct AssemblerState {
    first_pass: bool,

    scopes: Arena<Scope>,
    /// Stack of active scopes, always contains at least one item.
    active_scopes: Vec<ScopeId>,

    sections: Arena<Section>,
    section_names: HashMap<String, SectionId>,
    current_section: SectionId,
    current_pc: Word,
}

#[cfg_attr(test, automock)]
impl AssemblerState {
    /// Initialize the AssemblerState at the beginning of first pass
    pub fn new() -> AssemblerState {
        Default::default()
    }

    pub fn assemble(&mut self, files: &InputFiles) -> AsmResult<()> {
        for file_tokens in files.iter_file_tokens() {
            parse_file(self, file_tokens)?;
        }

        self.first_pass = false;

        for file_tokens in files.iter_file_tokens() {
            parse_file(self, file_tokens)?;
        }

        Ok(())
    }

    pub fn start_second_pass(&mut self) {
        self.first_pass = false;
    }

    pub(super) fn get_current_pc_symbol(
        &self,
        attached_scope: Option<ScopeId>,
        defined_at: Location,
    ) -> Symbol {
        Symbol::Location {
            section: self.current_section,
            offset: self.current_pc,
            attached_scope,
            defined_at,
        }
    }

    /// Define a symbol with the given value or check that it is already defined with identical value
    pub(super) fn define_symbol(&mut self, sym_name: &str, symbol: Symbol) -> AsmResult<()> {
        if sym_name.find(SCOPE_PATH_SEP).is_some() {
            return Err(AsmError::OtherError {
                description: "Symbol definition can't contain path separators".to_owned(),
            });
        }
        let active_scope = &mut self.scopes[*self.active_scopes.last().unwrap()].0;
        let previous_def = active_scope.get(sym_name);
        if self.first_pass {
            if let Some(previous_def) = previous_def {
                return Err(AsmError::SymbolRedefinition {
                    location: symbol.get_defined_at(),
                    previous_definition: previous_def.get_defined_at(),
                });
            } else {
                active_scope.insert(sym_name.into(), symbol);
            }
        } else {
            if previous_def != Some(&symbol) {
                Err(AsmError::SymbolChangedValue {
                    location: symbol.get_defined_at(),
                })?;
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

    pub(super) fn emit_word(&mut self, word: Word) {
        self.sections[self.current_section].data.push(word);
    }
}

impl Default for AssemblerState {
    fn default() -> AssemblerState {
        let mut scopes = Arena::new();
        let root_scope = scopes.alloc(Scope(HashMap::new()));
        let mut sections = Arena::new();
        let text_section = sections.alloc(Section {
            start_address: 0,
            data: Vec::new(),
        });

        AssemblerState {
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
        defined_at: Location,
    },
    /// Symbol that is just a value, not attached to anything
    Free { value: Value, defined_at: Location },
}

impl Symbol {
    pub fn get_value(&self, state: &AssemblerState) -> Value {
        match self {
            Symbol::Location {
                section,
                offset,
                attached_scope: _,
                defined_at: _,
            } => Value::from(state.sections[*section].start_address) + Value::from(*offset),
            Symbol::Free {
                value,
                defined_at: _,
            } => *value,
        }
    }

    pub fn get_defined_at(&self) -> Location {
        match self {
            Symbol::Location {
                section: _,
                offset: _,
                attached_scope: _,
                defined_at,
            } => defined_at.clone(),
            Symbol::Free {
                value: _,
                defined_at,
            } => defined_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Section {
    start_address: Word,
    data: Vec<Word>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AsmError {
    #[error("Unexpected token")]
    UnexpectedToken {
        expected: Cow<'static, str>,
        location: Location,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof { expected: Cow<'static, str> },
    #[error("Unexpected instruction mnemonic")]
    UnexpectedInstructionMnemonic { location: Location },
    #[error("Invalid general purpose register name format")]
    InvalidGprName { location: Location },
    #[error("Invalid control register name format")]
    InvalidCrName { location: Location },
    #[error("Value out of range")]
    ValueOutOfRange { location: Location },
    #[error("Undefined symbol")]
    UndefinedSymbol { location: Location },
    #[error("Negative shift ammount")]
    NegativeShiftAmmount { location: Location },
    #[error("Redefinition of symbol")]
    SymbolRedefinition {
        location: Location,
        previous_definition: Location,
    },
    #[error("Symbol changed value in second pass")]
    SymbolChangedValue { location: Location },

    #[error("IO error: {description}")]
    IoError { description: String },

    #[error("Other error: {description}")]
    OtherError { description: String },
}

impl From<AsmError> for Diagnostic<FileId> {
    fn from(e: AsmError) -> Diagnostic<FileId> {
        let ret = Diagnostic::error().with_message(format!("{}", e));
        match e {
            AsmError::UnexpectedToken { expected, location } => ret.with_labels(vec![location
                .to_primary_label()
                .with_message(format!("Expected {}", expected))]),
            _ => ret,
        }
    }
}

pub type AsmResult<V> = Result<V, AsmError>;
