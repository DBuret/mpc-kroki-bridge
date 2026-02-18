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
                "description": "Génère des graphiques de données (barres, lignes, etc.) via Vega-Lite JSON.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Spécification JSON Vega-Lite" }
                    },
                    "required": ["source"]
                }
            },
            {
                "name": "render_mermaid",
                "description": "Génère des diagrammes Mermaid (flowcharts, gantt, git graph).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Code Mermaid" }
                    },
                    "required": ["source"]
                }
            }
        ]
    })
}