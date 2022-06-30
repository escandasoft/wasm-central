use std::collections::HashMap;
use kafka::client::KafkaClient;

#[derive(Clone)]
pub enum Schema {
    Root(String, Vec<Schema>),
    Record(String, Vec<Schema>),
    Field(String, String),
    RECORD_END
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

type ObjectReader = dyn Iterator<Item=Schema>;

trait SchemaWriter
{
    fn write_record(&self, name: String, record: Schema, reader: &ObjectReader);
    fn write_field(&self, name: String, field: Schema, reader: &ObjectReader);
    fn end_record(&self);
}

trait TypingAdapter {
    fn adapt(&mut self, typing: Typing, reader: &ObjectReader, writer: &dyn SchemaWriter) {
        self.write(&typing.schema, reader, writer);
    }

    fn write(&mut self, schema: &Schema, reader: &ObjectReader, writer: &dyn SchemaWriter) {
        match schema {
            Schema::Root(name, records) => {
                writer.write_record(name.clone(), schema.clone(), reader);
            }
            Schema::Record(name, fields) => {
                writer.write_record(name.clone(), schema.clone(), reader);
                for field in fields {
                    self.write(field, reader, writer);
                }
            }
            Schema::Field(name, typing) => {
                writer.write_field(name.clone(), schema.clone(), reader);
            }
            Schema::RECORD_END => {
                writer.end_record()
            }
        }
    }
}

#[derive(Clone)]
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