use super::FunctionKind;
use super::Identifier;
use super::SyntaxError;
use crate::chunk::Chunk;
use crate::chunk::FunctionObject;
use crate::opcode::OpCode;
use std::mem::replace;
use std::ops::Range;

pub enum ResolvedVariable {
    Local(u16),
    Upvalue(u16),
    Global,
}

struct Local {
    identifier: Identifier,
    depth: usize,
    initialized: bool,
}

#[derive(PartialEq)]
pub(super) struct Upvalue {
    pub index: u16,
    pub is_local: bool,
}

struct LoopContext {
    stack_depth_at_start: usize,
    break_patches: Vec<usize>,
    loop_start: usize,
    enclosing: Option<Box<LoopContext>>,
}

pub(super) struct Compiler {
    pub(super) enclosing: Option<Box<Compiler>>,
    pub(super) function_object: FunctionObject,
    pub(super) upvalues: Vec<Upvalue>,
    locals: Vec<Local>,
    function_kind: FunctionKind,
    scope_depth: usize,
    loop_context: Option<LoopContext>,
}

impl Compiler {
    pub(super) fn new(function_object: FunctionObject, function_kind: FunctionKind) -> Self {
        Compiler {
            enclosing: None,
            function_object,
            function_kind,
            scope_depth: 0,
            loop_context: None,
            locals: Vec::new(),
            upvalues: Vec::new(),
        }
    }

    pub(super) fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.function_object.chunk
    }

    pub(super) fn is_top_level_code(&self) -> bool {
        matches!(self.function_kind, FunctionKind::Script)
    }

    pub(super) fn is_global_scope(&self) -> bool {
        self.scope_depth == 0
    }

    pub(super) fn stack_depth(&self) -> usize {
        self.locals.len()
    }

    pub(super) fn resolve_variable(
        &mut self,
        identifier: &Identifier,
    ) -> Result<ResolvedVariable, SyntaxError> {
        for (stack_index, local) in self.locals.iter().enumerate().rev() {
            if local.identifier.name.eq(&identifier.name) {
                if !local.initialized {
                    return Err(SyntaxError {
                        message: "Can't read local variable in its own initializer.".to_owned(),
                        span: identifier.span.clone(),
                    });
                } else {
                    return Ok(ResolvedVariable::Local(stack_index as u16));
                }
            }
        }
        match self.resolve_upvalue(identifier) {
            Some(index) => Ok(ResolvedVariable::Upvalue(index)),
            None => Ok(ResolvedVariable::Global),
        }
    }

    fn resolve_upvalue(&mut self, identifier: &Identifier) -> Option<u16> {
        let Some(enclosing) = &mut self.enclosing else {
            return None;
        };
        for (stack_index, local) in enclosing.locals.iter().enumerate() {
            if local.identifier.name.eq(&identifier.name) {
                let new_upvalue = Upvalue {
                    index: stack_index as u16,
                    is_local: true,
                };
                return Some(self.add_upvalue(new_upvalue));
            }
        }
        enclosing.resolve_upvalue(identifier).map(|upv_index| {
            self.add_upvalue(Upvalue {
                index: upv_index,
                is_local: false,
            })
        })
    }

    fn add_upvalue(&mut self, new_upvalue: Upvalue) -> u16 {
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if &new_upvalue == upvalue {
                return i as u16;
            }
        }
        self.upvalues.push(new_upvalue);
        self.function_object.upvalue_count += 1;
        (self.upvalues.len() - 1) as u16
    }

    pub(super) fn add_local(&mut self, identifier: &Identifier) -> Result<usize, SyntaxError> {
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.identifier.name.eq(&identifier.name) {
                return Err(SyntaxError {
                    message: "Already a variable with this name in this scope.".to_owned(),
                    span: identifier.span.clone(),
                });
            }
        }

        if self.locals.len() < 2usize.pow(16) {
            let local = Local {
                identifier: identifier.clone(),
                depth: self.scope_depth,
                initialized: false,
            };
            self.locals.push(local);
            Ok(self.locals.len() - 1)
        } else {
            panic!("Too many local variables in function.")
        }
    }

    pub(super) fn initialize_local(&mut self, local_index: usize) {
        self.locals[local_index].initialized = true;
    }

    pub(super) fn reserve_first_stack_slot(&mut self) {
        self.add_local(&Identifier::empty())
            .expect("this should be first local variable in context");
    }

    pub(super) fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    pub(super) fn end_scope(&mut self, span: &Range<usize>) {
        while self
            .locals
            .last()
            .is_some_and(|loc| loc.depth == self.scope_depth)
        {
            self.locals.pop().expect("locals.last() should be Some()");
            self.function_object
                .chunk
                .add_byte(OpCode::Pop, span.clone());
        }

        self.scope_depth -= 1;
    }

    pub(super) fn is_inside_loop(&self) -> bool {
        self.loop_context.is_some()
    }

    pub(super) fn enter_loop_context(&mut self, loop_start: usize) {
        let new_context = LoopContext {
            stack_depth_at_start: self.stack_depth(),
            break_patches: Vec::new(),
            loop_start,
            enclosing: None,
        };
        let enclosing = self.loop_context.replace(new_context);
        let Some(ref mut ctx) = self.loop_context else {
            panic!("loop_context should be Some()");
        };
        ctx.enclosing = enclosing.map(Box::new)
    }

    pub(super) fn exit_loop_context(&mut self) {
        let Some(ref mut ctx) = self.loop_context else {
            panic!("no loop context")
        };
        let enclosing = ctx.enclosing.take().map(|x| *x);
        let ctx =
            replace(&mut self.loop_context, enclosing).expect("loop context should be checked");

        for break_ in &ctx.break_patches {
            self.patch_jump(*break_);
        }
    }

    pub(super) fn compile_break(&mut self, span: Range<usize>) {
        let mut ctx = self.loop_context.take().expect("None should be checked");
        let delta_depth = self.stack_depth() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.emit_byte(OpCode::Pop, span.clone());
        }
        let jump = self.emit_jump(OpCode::Jump, span.clone());
        ctx.break_patches.push(jump);
        self.loop_context.replace(ctx);
    }

    pub(super) fn compile_continue(&mut self, span: Range<usize>) {
        let ctx = self.loop_context.take().expect("None should be checked");
        let delta_depth = self.stack_depth() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.emit_byte(OpCode::Pop, span.clone());
        }
        self.emit_loop(ctx.loop_start, span.clone());
        self.loop_context.replace(ctx);
    }

    pub(super) fn rebase_loop_start(&mut self, new_loop_start: usize) {
        self.loop_context
            .as_mut()
            .expect("Loop context rebase should be done inside loop")
            .loop_start = new_loop_start;
    }
}
