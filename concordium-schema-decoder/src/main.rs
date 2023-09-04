mod schema;

use base64::Engine;
use schema::{parse_schema, schema_to_json};

#[macro_use]
extern crate rocket;

#[post("/", data = "<data>")]
fn schema2json(data: String) -> String {
    // TODO Take schema version as query param.
    // TODO Do proper error handling.
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .unwrap();
    let schema = parse_schema(None, &bytes).unwrap();

    let json = schema_to_json(&schema).unwrap();
    serde_json::to_string(&json).unwrap()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![schema2json])
}
