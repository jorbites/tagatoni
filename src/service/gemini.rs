use crate::model::recipe::Recipe;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

pub struct GeminiService {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Serialize)]
struct GeminiRequest {
    model: String,
    input: String,
    generation_config: GenerationConfig,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f64,
    max_output_tokens: i64,
    top_p: f64,
    thinking_level: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    mime_type: String,
    schema: Value,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    steps: Option<Vec<Step>>,
}

#[derive(Deserialize, Debug)]
struct Step {
    #[serde(rename = "type")]
    step_type: String,
    content: Option<Value>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AuditResult {
    pub calories: Option<i32>,
    #[serde(rename = "recipeCuisine")]
    pub recipe_cuisine: Option<String>,
    #[serde(rename = "recipeYield")]
    pub recipe_yield: Option<i32>,
    #[serde(rename = "ganttTable")]
    pub gantt_table: Option<Value>,
}

pub fn build_prompt_and_schema(recipe: &Recipe) -> (String, Value) {
    let need_calories = recipe.calories.is_none();
    let need_cuisine = recipe.recipe_cuisine.is_none();
    let need_yield = recipe.recipe_yield.is_none();
    let need_gantt = recipe.gantt_table.is_none();

    let mut request_items = Vec::new();

    if need_calories {
        request_items.push(format!(
            "{}. Calories (an integer representing estimated calories per serving).",
            request_items.len() + 1
        ));
    }

    if need_cuisine {
        request_items.push(format!(
            "{}. Recipe cuisine (must be selected from the allowed enum list).",
            request_items.len() + 1
        ));
    }

    if need_yield {
        request_items.push(format!(
            "{}. Recipe yield (an integer representing the estimated number of servings, e.g. 4).",
            request_items.len() + 1
        ));
    }

    if need_gantt {
        request_items.push(format!(
            "{}. Gantt Table — a visual cooking-flow table organizing the recipe as a grid.
rows MUST contain an entry for EVERY ingredient in the recipe's ingredients array, maintaining the exact same order (0-indexed).
Columns represent sequential cooking actions (e.g., \"melt\", \"mix\", \"fold in\", \"bake\").
Each action cell spans a range of ingredient rows [startRow, endRow] (INCLUSIVE, e.g. [0, 2] means rows 0, 1, and 2 participate).
Group ingredients by their first shared action step (integer group ID starting at 0).
Use the same language as the recipe for all text (preSteps, ingredient names, action labels).
Return object structure:
- preSteps: array of preparation step strings (e.g. \"Preheat oven to 350°F\")
- rows: array of objects {{ \"ingredient\": string, \"group\": integer }}
- columns: array of objects {{ \"action\": string, \"rowSpan\": [startRow, endRow], \"colIndex\": integer }}",
            request_items.len() + 1
        ));
    }

    let items_text = request_items.join("\n");

    let prompt = format!(
        "Audit this recipe and estimate the following missing field(s):\n{}\n\nRecipe Details:\nTitle: {}\nDescription: {}\nCategories: {}\nIngredients:\n{}\nSteps:\n{}",
        items_text,
        recipe.title,
        recipe.description,
        recipe.categories.join(", "),
        recipe
            .ingredients
            .iter()
            .map(|i| format!("- {}", i))
            .collect::<Vec<_>>()
            .join("\n"),
        recipe
            .steps
            .iter()
            .enumerate()
            .map(|(idx, s)| format!("{}. {}", idx + 1, s))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let mut properties = json!({});
    let mut property_ordering = Vec::new();
    let mut required = Vec::new();

    if need_calories {
        properties["calories"] = json!({ "type": "integer" });
        property_ordering.push("calories");
        required.push("calories");
    }

    if need_cuisine {
        properties["recipeCuisine"] = json!({
            "type": "string",
            "enum": [
                "Spanish", "Catalan", "Italian", "Mexican", "Japanese", "Chinese", "Indian",
                "French", "American", "Mediterranean", "Middle Eastern", "Greek", "Thai",
                "Vietnamese", "Moroccan", "Turkish", "Latin American", "Caribbean", "Nordic",
                "British", "German", "Eastern European", "African", "Asian Fusion", "International"
            ]
        });
        property_ordering.push("recipeCuisine");
        required.push("recipeCuisine");
    }

    if need_yield {
        properties["recipeYield"] = json!({ "type": "integer" });
        property_ordering.push("recipeYield");
        required.push("recipeYield");
    }

    if need_gantt {
        properties["ganttTable"] = json!({
            "type": "object",
            "properties": {
                "preSteps": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "rows": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "ingredient": { "type": "string" },
                            "group": { "type": "integer" }
                        },
                        "required": ["ingredient", "group"]
                    }
                },
                "columns": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "action": { "type": "string" },
                            "rowSpan": {
                                "type": "array",
                                "items": { "type": "integer" },
                                "minItems": 2,
                                "maxItems": 2
                            },
                            "colIndex": { "type": "integer" }
                        },
                        "required": ["action", "rowSpan", "colIndex"]
                    }
                }
            },
            "required": ["preSteps", "rows", "columns"]
        });
        property_ordering.push("ganttTable");
        required.push("ganttTable");
    }

    let response_schema = json!({
        "type": "object",
        "properties": properties,
        "propertyOrdering": property_ordering,
        "required": required
    });

    (prompt, response_schema)
}

impl GeminiService {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_default();

        Self { api_key, client }
    }

    /// Calls the Gemini Interactions API to audit a recipe and extract missing SEO metadata.
    pub async fn audit_recipe(&self, recipe: &Recipe) -> Result<AuditResult, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/interactions?key={}",
            self.api_key
        );

        let (prompt, response_schema) = build_prompt_and_schema(recipe);

        let payload = GeminiRequest {
            model: "models/gemini-3.5-flash".to_string(),
            input: prompt,
            generation_config: GenerationConfig {
                temperature: 1.0,
                max_output_tokens: 65536,
                top_p: 0.95,
                thinking_level: "minimal".to_string(),
            },
            response_format: ResponseFormat {
                format_type: "text".to_string(),
                mime_type: "application/json".to_string(),
                schema: response_schema,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API error ({}): {}", status, error_body));
        }

        let gemini_resp: GeminiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

        let steps = gemini_resp
            .steps
            .ok_or_else(|| "No steps found in Gemini response".to_string())?;

        let text_content = extract_text_from_steps(&steps).ok_or_else(|| {
            "Could not find model output text in Gemini interaction steps".to_string()
        })?;

        let audit_result: AuditResult = serde_json::from_str(&text_content).map_err(|e| {
            format!(
                "Failed to deserialize model output JSON schema: {}. Raw text: {}",
                e, text_content
            )
        })?;

        Ok(audit_result)
    }
}

fn extract_text_from_steps(steps: &[Step]) -> Option<String> {
    for step in steps {
        if step.step_type == "model_output" {
            if let Some(content_val) = &step.content {
                if let Some(arr) = content_val.as_array() {
                    for part in arr {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            return Some(text.to_string());
                        }
                    }
                } else if let Some(text) = content_val.as_str() {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::oid::ObjectId;
    use serde_json::json;

    #[test]
    fn test_extract_text_from_steps_array() {
        let steps = vec![
            Step {
                step_type: "thought".to_string(),
                content: Some(json!("Thinking process...")),
            },
            Step {
                step_type: "model_output".to_string(),
                content: Some(json!([
                    {
                        "type": "text",
                        "text": "{\"calories\": 350, \"recipeCuisine\": \"Italian\", \"recipeYield\": 4}"
                    }
                ])),
            },
        ];

        let text = extract_text_from_steps(&steps);
        assert_eq!(
            text,
            Some(
                "{\"calories\": 350, \"recipeCuisine\": \"Italian\", \"recipeYield\": 4}"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_extract_text_from_steps_string() {
        let steps = vec![Step {
            step_type: "model_output".to_string(),
            content: Some(json!(
                "{\"calories\": 450, \"recipeCuisine\": \"Mexican\", \"recipeYield\": 6}"
            )),
        }];

        let text = extract_text_from_steps(&steps);
        assert_eq!(
            text,
            Some(
                "{\"calories\": 450, \"recipeCuisine\": \"Mexican\", \"recipeYield\": 6}"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_audit_result_deserialization() {
        let raw_json = "{\"calories\": 500, \"recipeCuisine\": \"Spanish\", \"recipeYield\": 2}";
        let result: AuditResult = serde_json::from_str(raw_json).unwrap();
        assert_eq!(result.calories, Some(500));
        assert_eq!(result.recipe_cuisine, Some("Spanish".to_string()));
        assert_eq!(result.recipe_yield, Some(2));
        assert_eq!(result.gantt_table, None);
    }

    #[test]
    fn test_audit_result_deserialization_with_gantt_table() {
        let raw_json =
            "{\"ganttTable\": {\"preSteps\": [\"Heat oven\"], \"rows\": [], \"columns\": []}}";
        let result: AuditResult = serde_json::from_str(raw_json).unwrap();
        assert!(result.gantt_table.is_some());
        assert_eq!(result.gantt_table.unwrap()["preSteps"][0], "Heat oven");
    }

    #[test]
    fn test_audit_result_partial_deserialization() {
        let raw_json = "{\"recipeCuisine\": \"Italian\"}";
        let result: AuditResult = serde_json::from_str(raw_json).unwrap();
        assert_eq!(result.calories, None);
        assert_eq!(result.recipe_cuisine, Some("Italian".to_string()));
        assert_eq!(result.recipe_yield, None);
        assert_eq!(result.gantt_table, None);
    }

    #[test]
    fn test_build_prompt_and_schema_single_missing_field() {
        let recipe = Recipe {
            id: ObjectId::new(),
            title: "Paella".to_string(),
            description: "Traditional Spanish dish".to_string(),
            categories: vec!["Rice".to_string()],
            ingredients: vec!["Rice".to_string(), "Saffron".to_string()],
            steps: vec!["Cook rice".to_string()],
            calories: Some(600),
            recipe_cuisine: None, // missing
            recipe_yield: Some(4),
            gantt_table: Some(json!({})),
        };

        let (prompt, schema) = build_prompt_and_schema(&recipe);
        assert!(prompt.contains("1. Recipe cuisine (must be selected from the allowed enum list)."));
        assert!(!prompt.contains("Calories"));
        assert!(!prompt.contains("Recipe yield"));
        assert!(!prompt.contains("Gantt Table"));

        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 1);
        assert_eq!(req_array[0], "recipeCuisine");
        assert!(schema["properties"].get("calories").is_none());
        assert!(schema["properties"].get("recipeYield").is_none());
        assert!(schema["properties"].get("recipeCuisine").is_some());
        assert!(schema["properties"].get("ganttTable").is_none());
    }

    #[test]
    fn test_build_prompt_and_schema_gantt_only() {
        let recipe = Recipe {
            id: ObjectId::new(),
            title: "Paella".to_string(),
            description: "Traditional Spanish dish".to_string(),
            categories: vec!["Rice".to_string()],
            ingredients: vec!["Rice".to_string(), "Saffron".to_string()],
            steps: vec!["Cook rice".to_string()],
            calories: Some(600),
            recipe_cuisine: Some("Spanish".to_string()),
            recipe_yield: Some(4),
            gantt_table: None, // missing
        };

        let (prompt, schema) = build_prompt_and_schema(&recipe);
        assert!(prompt.contains("1. Gantt Table"));
        assert!(!prompt.contains("Calories"));
        assert!(!prompt.contains("Recipe cuisine"));
        assert!(!prompt.contains("Recipe yield"));

        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 1);
        assert_eq!(req_array[0], "ganttTable");
        assert!(schema["properties"].get("ganttTable").is_some());
    }

    #[test]
    fn test_build_prompt_and_schema_multiple_missing_fields() {
        let recipe = Recipe {
            id: ObjectId::new(),
            title: "Tacos".to_string(),
            description: "Tasty tacos".to_string(),
            categories: vec![],
            ingredients: vec!["Tortilla".to_string()],
            steps: vec!["Assemble".to_string()],
            calories: None,       // missing
            recipe_cuisine: None, // missing
            recipe_yield: None,   // missing
            gantt_table: None,    // missing
        };

        let (prompt, schema) = build_prompt_and_schema(&recipe);
        assert!(prompt.contains("1. Calories"));
        assert!(prompt.contains("2. Recipe cuisine"));
        assert!(prompt.contains("3. Recipe yield"));
        assert!(prompt.contains("4. Gantt Table"));

        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 4);
        assert!(schema["properties"].get("calories").is_some());
        assert!(schema["properties"].get("recipeCuisine").is_some());
        assert!(schema["properties"].get("recipeYield").is_some());
        assert!(schema["properties"].get("ganttTable").is_some());
    }
}
