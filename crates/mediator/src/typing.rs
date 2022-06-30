use std::collections::HashMap;
use kafka::client::KafkaClient;

pub enum Schema {
    Root(String, Vec<Schema>),
    Record(String, Vec<Schema>),
    Field(String, String),
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum TypingKind {
    In, Out
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct TypingId {
    pub name: String,
    pub kind: TypingKind,
}

trait SchemaWriter<R, F>
where
    R: FnOnce(Schema) -> (),
    F: FnOnce(Schema) -> ()
{
    fn write_record(&self, name: String, record: Schema, value: R);
    fn write_field(&self, name: String, field: Schema, field: F);
    fn end_record(&self);
}

struct TypingAdapter;

impl TypingAdapter {
    pub fn adapt<R, F>(&mut self, typing: &Typing, writer: &dyn SchemaWriter<R, F>) {

    }
}

pub struct Typing {
    pub schema: Schema
}

pub struct TypingRegistry {
    typings: HashMap<TypingId, Typing>,
}

impl TypingRegistry {
    pub fn new() -> TypingRegistry {
        TypingRegistry {
            typings: HashMap::new()
        }
    }

    pub fn add_type(&mut self, name: String, kind: TypingKind, schema: Schema) {
        self.typings.insert(TypingId { name, kind }, Typing { schema });
    }
}