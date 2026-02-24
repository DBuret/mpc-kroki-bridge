use serde_json::{json, Value};

pub fn get_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "render_plantuml",
                "description": "Génère des schémas d'architecture, séquences et classes via PlantUML.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Code PlantUML (ex: @startuml...)" }
                    },
                    "required": ["source"]
                }
            },
            {
                "name": "render_vega",
                "description": "Generates data charts (bar, line, pie, etc.) using a Vega-Lite JSON specification. IMPORTANT: the provided JSON must be strictly valid. All numeric values must be pre-computed literals (e.g. 10521.96). Arithmetic expressions like '4332.57 + 6189.39' are FORBIDDEN and will cause an error. Aggregate and compute all values BEFORE building the JSON spec.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "A strictly valid Vega-Lite JSON specification. All numeric values must be literals (number type), never expressions. The JSON must be directly parseable as-is."
                        }
                    },
                    "required": ["source"]
                }
            }
        ]
    })
}