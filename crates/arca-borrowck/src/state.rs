//! Ownership state tracking and borrow state definitions for Arca.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarState {
    Active,
    Moved,
    Borrowed { count: usize, is_mut: bool },
}

#[derive(Debug, Clone)]
pub struct ScopeOwnership {
    pub vars: HashMap<String, VarState>,
}

impl ScopeOwnership {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OwnershipTracker {
    scopes: Vec<ScopeOwnership>,
}

impl OwnershipTracker {
    pub fn new() -> Self {
        Self {
            scopes: vec![ScopeOwnership::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(ScopeOwnership::new());
    }

    pub fn pop_scope(&mut self) -> Vec<String> {
        let mut drops = Vec::new();
        if let Some(scope) = self.scopes.pop() {
            for (vname, state) in scope.vars {
                if state == VarState::Active {
                    drops.push(vname);
                }
            }
        }
        drops
    }

    pub fn declare_var(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name, VarState::Active);
        }
    }

    pub fn mark_moved(&mut self, name: &str) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(state) = scope.vars.get_mut(name) {
                if *state == VarState::Moved {
                    return false; // Already moved!
                }
                *state = VarState::Moved;
                return true;
            }
        }
        false
    }

    pub fn check_used(&self, name: &str) -> Result<(), String> {
        for scope in self.scopes.iter().rev() {
            if let Some(state) = scope.vars.get(name) {
                if *state == VarState::Moved {
                    return Err(format!(
                        "Use of moved value '{}'. Value was previously moved.",
                        name
                    ));
                }
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn borrow_var(&mut self, name: &str, is_mut: bool) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(state) = scope.vars.get_mut(name) {
                match state {
                    VarState::Moved => {
                        return Err(format!("Cannot borrow moved value '{}'", name));
                    }
                    VarState::Active => {
                        *state = VarState::Borrowed {
                            count: 1,
                            is_mut,
                        };
                        return Ok(());
                    }
                    VarState::Borrowed { count, is_mut: existing_mut } => {
                        if is_mut || *existing_mut {
                            return Err(format!(
                                "Cannot borrow '{}' as mutable because it is already borrowed",
                                name
                            ));
                        }
                        *count += 1;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}
