use vrl::compiler::{Function, expression::Expr};
use vrl::value::{ObjectMap, Value};
use vrl::prelude::*;

use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::DefaultHasher;
use std::sync::Mutex;
use std::time::Instant;
use std::hash::Hash;
use std::hash::Hasher;
use std::time::Duration;

fn open_context(
) -> Resolved {
    Ok(Value::Object(Default::default()))
}

struct GlobalContext {
    contexts: HashMap<u64, SingleContext>
}

impl GlobalContext {
    fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }
}

struct SingleContext {
    data: Value,
    until: Instant,
}

static GLOBAL_CONTEXT: Mutex<Option<GlobalContext>> = Mutex::new(None);

#[derive(Clone, Copy, Debug)]
pub struct OpenContext;
impl Function for OpenContext {
    fn identifier(&self) -> &'static str {
        "open_context"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                keyword: "keys",
                kind: kind::ARRAY,
                required: true,
            },
            Parameter {
                keyword: "seconds",
                kind: kind::INTEGER,
                required: true,
            },
        ]
    }

    fn examples(&self) -> &'static [Example] {
        &[Example {
            title: "open context",
            source: r#"open_context(["test"], 5)"#,
            result: Ok(r#"{"key": 8194875, "data": {}}"#),
        }]
    }

    fn compile(
        &self,
        state: &TypeState,
        ctx: &mut FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Compiled {
        let keys = arguments.required_array("keys")?;
        let timeout_val = arguments.required_literal("seconds", state)?;
        let Value::Integer(timeout) = timeout_val else {
            panic!("Timeout must be integer");
        };

        Ok(OpenContextFn {
            keys,
            timeout
        }
        .as_expr())
    }
}

#[derive(Debug, Clone)]
pub struct OpenContextFn {
    keys: Vec<Expr>,
    timeout: i64,
}

impl FunctionExpression for OpenContextFn {
    fn resolve(&self, ctx: &mut Context) -> Resolved {
        let keys = self.keys.iter().map(|e| e.resolve(ctx)).collect::<Result<Vec<Value>, ExpressionError>>()?;
        let mut hasher = DefaultHasher::new();
        keys.hash(&mut hasher);
        let hash = hasher.finish();

        let mut global = GLOBAL_CONTEXT.lock().unwrap();
        if global.is_none() {
            *global = Some(GlobalContext::new())
        }
        let global = global.as_mut().unwrap();
        let entry = global.contexts.entry(hash).or_insert_with(|| SingleContext {
            data: Value::Object(ObjectMap::default()),
            until: Instant::now() + Duration::from_secs(self.timeout.try_into().unwrap()),
        });

        let mut ret = ObjectMap::new();
        ret.insert("key".into(), Value::Integer(hash as i64));
        ret.insert("data".into(), entry.data.clone());

        Ok(Value::Object(ret))
    }

    fn type_def(&self, _: &TypeState) -> TypeDef {
        TypeDef::object(Collection::any())
    }
}


#[derive(Clone, Copy, Debug)]
pub struct UpdateContext;
impl Function for UpdateContext {
    fn identifier(&self) -> &'static str {
        "update_context"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                keyword: "context",
                kind: kind::OBJECT,
                required: true,
            },
        ]
    }

    fn examples(&self) -> &'static [Example] {
        &[Example {
            title: "update context",
            source: r#"update_context({"key": 8194875, "data": { "hi": 5 }})"#,
            result: Ok(r#"null"#),
        }]
    }

    fn compile(
        &self,
        state: &TypeState,
        ctx: &mut FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Compiled {
        let context = arguments.required_expr("context");

        Ok(UpdateContextFn {
            context
        }
        .as_expr())
    }
}

#[derive(Debug, Clone)]
pub struct UpdateContextFn {
    context: Expr,
}

impl FunctionExpression for UpdateContextFn {
    fn resolve(&self, ctx: &mut Context) -> Resolved {
        let Value::Object(context) = self.context.resolve(ctx)? else {
            panic!("Expected context to be an object");
        };
        let Value::Integer(key_value) = context.get("key").unwrap() else {
            panic!("Expected key to be integer");
        };

        let mut global = GLOBAL_CONTEXT.lock().unwrap();
        if global.is_none() {
            *global = Some(GlobalContext::new())
        }
        let global = global.as_mut().unwrap();
        global.contexts.insert(*key_value as u64, SingleContext {
            data: context.get("data").unwrap().clone(),
            until: Instant::now(),
        });

        Ok(Value::Null)
    }

    fn type_def(&self, _: &TypeState) -> TypeDef {
        TypeDef::null()
    }
}
