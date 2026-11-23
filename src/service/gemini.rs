use crate::model::recipe::Recipe;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::Duration;

pub const ALLOWED_DIFFICULTIES: &[&str] = &["Easy", "Medium", "Hard"];

pub const ALLOWED_DIETARY_TAGS: &[&str] = &[
    "vegan",
    "vegetarian",
    "gluten-free",
    "dairy-free",
    "nut-free",
    "egg-free",
    "soy-free",
    "sugar-free",
    "low-carb",
    "high-protein",
    "low-fat",
    "keto",
    "paleo",
];

pub const ALLOWED_CUISINES: &[&str] = &[
    "Spanish",
    "Catalan",
    "Italian",
    "Mexican",
    "Japanese",
    "Chinese",
    "Indian",
    "French",
    "American",
    "Mediterranean",
    "Middle Eastern",
    "Greek",
    "Thai",
    "Vietnamese",
    "Moroccan",
    "Turkish",
    "Latin American",
    "Caribbean",
    "Nordic",
    "British",
    "German",
    "Eastern European",
    "African",
    "Asian Fusion",
    "International",
];

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

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AuditResult {
    pub calories: Option<i32>,
    #[serde(rename = "recipeCuisine")]
    pub recipe_cuisine: Option<String>,
    #[serde(rename = "recipeYield")]
    pub recipe_yield: Option<i32>,
    #[serde(rename = "ganttTable")]
    pub gantt_table: Option<Value>,
    pub difficulty: Option<String>,
    pub protein: Option<i32>,
    pub carbohydrates: Option<i32>,
    pub fat: Option<i32>,
    #[serde(rename = "saturatedFat")]
    pub saturated_fat: Option<i32>,
    pub fiber: Option<i32>,
    pub sugar: Option<i32>,
    pub sodium: Option<i32>,
    #[serde(rename = "dietaryTags")]
    pub dietary_tags: Option<Vec<String>>,
}

pub fn build_prompt_and_schema(recipe: &Recipe) -> (String, Value) {
    let need_calories = recipe.calories.is_none();
    let need_cuisine = recipe.recipe_cuisine.is_none();
    let need_yield = recipe.recipe_yield.is_none();
    let need_gantt = recipe.gantt_table.is_none();
    let need_difficulty = recipe.difficulty.is_none();
    let need_protein = recipe.protein.is_none();
    let need_carbs = recipe.carbohydrates.is_none();
    let need_fat = recipe.fat.is_none();
    let need_saturated_fat = recipe.saturated_fat.is_none();
    let need_fiber = recipe.fiber.is_none();
    let need_sugar = recipe.sugar.is_none();
    let need_sodium = recipe.sodium.is_none();
    let need_dietary_tags = recipe.needs_dietary_tags();

    let mut request_items = Vec::new();

    if need_calories {
        request_items.push(format!(
            "{}. Calories (a non-negative integer representing estimated kcal per serving).",
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
            "{}. Recipe yield (a positive integer representing the estimated number of servings, e.g. 4).",
            request_items.len() + 1
        ));
    }

    if need_difficulty {
        request_items.push(format!(
            "{}. Difficulty (must be one of \"Easy\", \"Medium\", or \"Hard\" based on culinary technique complexity, active effort, and precision required: Easy = basic techniques and forgiving timing; Medium = some skill, timing/temperature coordination, or multi-stage prep; Hard = advanced techniques like tempering/emulsions/complex pastry or intricate assembly).",
            request_items.len() + 1
        ));
    }

    let need_any_macro = need_protein
        || need_carbs
        || need_fat
        || need_saturated_fat
        || need_fiber
        || need_sugar
        || need_sodium;

    if need_protein {
        request_items.push(format!(
            "{}. Protein (a non-negative integer in grams per serving).",
            request_items.len() + 1
        ));
    }

    if need_carbs {
        request_items.push(format!(
            "{}. Carbohydrates (a non-negative integer in total grams per serving).",
            request_items.len() + 1
        ));
    }

    if need_fat {
        request_items.push(format!(
            "{}. Fat (a non-negative integer in total grams per serving).",
            request_items.len() + 1
        ));
    }

    if need_saturated_fat {
        request_items.push(format!(
            "{}. Saturated fat (a non-negative integer in grams per serving; must be <= total fat).",
            request_items.len() + 1
        ));
    }

    if need_fiber {
        request_items.push(format!(
            "{}. Fiber (a non-negative integer in grams per serving; must be <= total carbohydrates).",
            request_items.len() + 1
        ));
    }

    if need_sugar {
        request_items.push(format!(
            "{}. Sugar (a non-negative integer in grams per serving; must be <= total carbohydrates).",
            request_items.len() + 1
        ));
    }

    if need_sodium {
        request_items.push(format!(
            "{}. Sodium (a non-negative integer in milligrams per serving).",
            request_items.len() + 1
        ));
    }

    if need_dietary_tags {
        request_items.push(format!(
            "{}. Dietary tags (array of applicable canonical tags from the allowed enum list; return [] if none apply). Apply strictly based on the recipe's ingredients and per-serving nutrition:
- \"vegan\": no animal products at all (no meat, fish, dairy, eggs, honey).
- \"vegetarian\": no meat, poultry, fish, or seafood.
- \"gluten-free\": no wheat, barley, rye, regular oats, bread, regular pasta, or gluten-containing soy sauce.
- \"dairy-free\": no milk, butter, cream, cheese, yogurt, ghee, or dairy derivatives.
- \"nut-free\": no tree nuts or peanuts.
- \"egg-free\": no eggs, yolks, whites, or egg-based mayo.
- \"soy-free\": no soy, tofu, tempeh, edamame, soy sauce, or tamari.
- \"sugar-free\": <= 1g sugar per serving and no added sugar/honey/syrup.
- \"low-carb\": <= 20g net carbs (carbohydrates minus fiber) per serving.
- \"high-protein\": >= 20g protein per serving (or >= 25% of calories from protein).
- \"low-fat\": <= 5g total fat per serving.
- \"keto\": <= 10g net carbs per serving with high fat and no grains/starches/sugar.
- \"paleo\": whole unprocessed foods only; no grains, legumes, dairy, refined sugar, or seed oils.",
            request_items.len() + 1
        ));
    }

    if need_gantt {
        request_items.push(format!(
            "{}. Gantt Table — a visual cooking-flow table organizing the recipe as a grid.
rows MUST contain an entry for EVERY ingredient in the recipe's ingredients array, maintaining the exact same order (0-indexed).
Columns represent sequential cooking actions (e.g., \"batir\", \"añadir y mezclar\", \"hornear 75 min a 180°C\").
Each action cell spans a range of ingredient rows [startRow, endRow] (INCLUSIVE, e.g. [0, 2] means rows 0, 1, and 2 participate).
Group ingredients by their first shared action step (integer group ID starting at 0).
CRITICAL FOR ACTION TEXT CONCISENESS:
- Action labels MUST be short and concise (1 to 4 words max).
- DO NOT repeat or list ingredient names in the action text! The cell rowSpan already visually links the action to its ingredients.
  * BAD: \"añadir harina, azúcares, levadura, canela y sal y volver a batir\"
  * GOOD: \"añadir y batir\" or \"mezclar y batir\"
  * BAD: \"añadir zanahoria y nueces\"
  * GOOD: \"incorporar y batir\"
- Use the exact same language as the recipe for all text (preSteps, ingredient names, action labels).
Return object structure:
- preSteps: array of preparation step strings (e.g. \"Preheat oven to 350°F\")
- rows: array of objects {{ \"ingredient\": string, \"group\": integer }}
- columns: array of objects {{ \"action\": string, \"rowSpan\": [startRow, endRow], \"colIndex\": integer }}",
            request_items.len() + 1
        ));
    }

    let items_text = request_items.join("\n");

    let mut existing_meta = Vec::new();
    if let Some(minutes) = recipe.minutes {
        existing_meta.push(format!("Total Time: {} minutes", minutes));
    }
    if let Some(method) = &recipe.method {
        existing_meta.push(format!("Cooking Method: {}", method));
    }
    if let Some(y) = recipe.recipe_yield {
        existing_meta.push(format!("Existing Servings (yield): {}", y));
    }
    if let Some(cal) = recipe.calories {
        existing_meta.push(format!("Existing Calories per serving: {} kcal", cal));
    }

    let existing_meta_section = if existing_meta.is_empty() {
        String::new()
    } else {
        let consistency_note = if need_any_macro
            && (recipe.calories.is_some() || recipe.recipe_yield.is_some())
        {
            "\nNote: Keep all per-serving nutritional estimates mathematically consistent with the existing servings and calories above (protein*4 + carbohydrates*4 + fat*9 ≈ calories, saturatedFat <= fat, sugar + fiber <= carbohydrates)."
        } else {
            ""
        };
        format!(
            "\nExisting Metadata:\n{}{}",
            existing_meta.join("\n"),
            consistency_note
        )
    };

    let prompt = format!(
        "Audit this recipe and estimate the following missing field(s):\n{}\n\nRecipe Details:\nTitle: {}\nDescription: {}\nCategories: {}{}\nIngredients:\n{}\nSteps:\n{}",
        items_text,
        recipe.title,
        recipe.description,
        recipe.categories.join(", "),
        existing_meta_section,
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
            "enum": ALLOWED_CUISINES
        });
        property_ordering.push("recipeCuisine");
        required.push("recipeCuisine");
    }

    if need_yield {
        properties["recipeYield"] = json!({ "type": "integer" });
        property_ordering.push("recipeYield");
        required.push("recipeYield");
    }

    if need_difficulty {
        properties["difficulty"] = json!({
            "type": "string",
            "enum": ALLOWED_DIFFICULTIES
        });
        property_ordering.push("difficulty");
        required.push("difficulty");
    }

    if need_protein {
        properties["protein"] = json!({ "type": "integer" });
        property_ordering.push("protein");
        required.push("protein");
    }

    if need_carbs {
        properties["carbohydrates"] = json!({ "type": "integer" });
        property_ordering.push("carbohydrates");
        required.push("carbohydrates");
    }

    if need_fat {
        properties["fat"] = json!({ "type": "integer" });
        property_ordering.push("fat");
        required.push("fat");
    }

    if need_saturated_fat {
        properties["saturatedFat"] = json!({ "type": "integer" });
        property_ordering.push("saturatedFat");
        required.push("saturatedFat");
    }

    if need_fiber {
        properties["fiber"] = json!({ "type": "integer" });
        property_ordering.push("fiber");
        required.push("fiber");
    }

    if need_sugar {
        properties["sugar"] = json!({ "type": "integer" });
        property_ordering.push("sugar");
        required.push("sugar");
    }

    if need_sodium {
        properties["sodium"] = json!({ "type": "integer" });
        property_ordering.push("sodium");
        required.push("sodium");
    }

    if need_dietary_tags {
        properties["dietaryTags"] = json!({
            "type": "array",
            "items": {
                "type": "string",
                "enum": ALLOWED_DIETARY_TAGS
            }
        });
        property_ordering.push("dietaryTags");
        required.push("dietaryTags");
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

/// Validates that Gemini returned every requested missing field with valid values,
/// and sanitizes `dietaryTags` (filtering to allowed tags and deduplicating).
pub fn validate_and_sanitize_result(
    recipe: &Recipe,
    mut result: AuditResult,
) -> Result<AuditResult, String> {
    if recipe.calories.is_none() && result.calories.is_none() {
        return Err("Missing required field in Gemini output: calories".to_string());
    }
    if recipe.recipe_cuisine.is_none() && result.recipe_cuisine.is_none() {
        return Err("Missing required field in Gemini output: recipeCuisine".to_string());
    }
    if recipe.recipe_yield.is_none() && result.recipe_yield.is_none() {
        return Err("Missing required field in Gemini output: recipeYield".to_string());
    }
    if recipe.difficulty.is_none() && result.difficulty.is_none() {
        return Err("Missing required field in Gemini output: difficulty".to_string());
    }
    if recipe.protein.is_none() && result.protein.is_none() {
        return Err("Missing required field in Gemini output: protein".to_string());
    }
    if recipe.carbohydrates.is_none() && result.carbohydrates.is_none() {
        return Err("Missing required field in Gemini output: carbohydrates".to_string());
    }
    if recipe.fat.is_none() && result.fat.is_none() {
        return Err("Missing required field in Gemini output: fat".to_string());
    }
    if recipe.saturated_fat.is_none() && result.saturated_fat.is_none() {
        return Err("Missing required field in Gemini output: saturatedFat".to_string());
    }
    if recipe.fiber.is_none() && result.fiber.is_none() {
        return Err("Missing required field in Gemini output: fiber".to_string());
    }
    if recipe.sugar.is_none() && result.sugar.is_none() {
        return Err("Missing required field in Gemini output: sugar".to_string());
    }
    if recipe.sodium.is_none() && result.sodium.is_none() {
        return Err("Missing required field in Gemini output: sodium".to_string());
    }
    if recipe.needs_dietary_tags() && result.dietary_tags.is_none() {
        return Err("Missing required field in Gemini output: dietaryTags".to_string());
    }
    if recipe.gantt_table.is_none() && result.gantt_table.is_none() {
        return Err("Missing required field in Gemini output: ganttTable".to_string());
    }

    if let Some(ref cuisine) = result.recipe_cuisine {
        if !ALLOWED_CUISINES.contains(&cuisine.as_str()) {
            return Err(format!("Invalid recipeCuisine value: {}", cuisine));
        }
    }

    if let Some(ref difficulty) = result.difficulty {
        if !ALLOWED_DIFFICULTIES.contains(&difficulty.as_str()) {
            return Err(format!("Invalid difficulty value: {}", difficulty));
        }
    }

    if let Some(y) = result.recipe_yield {
        if y <= 0 {
            return Err(format!("Invalid non-positive recipeYield value: {}", y));
        }
    }

    for (name, val) in [
        ("calories", result.calories),
        ("protein", result.protein),
        ("carbohydrates", result.carbohydrates),
        ("fat", result.fat),
        ("saturatedFat", result.saturated_fat),
        ("fiber", result.fiber),
        ("sugar", result.sugar),
        ("sodium", result.sodium),
    ] {
        if let Some(v) = val {
            if v < 0 {
                return Err(format!("Invalid negative {} value: {}", name, v));
            }
        }
    }

    if let Some(tags) = result.dietary_tags.take() {
        let mut seen = HashSet::new();
        let sanitized: Vec<String> = tags
            .into_iter()
            .filter(|tag| ALLOWED_DIETARY_TAGS.contains(&tag.as_str()))
            .filter(|tag| seen.insert(tag.clone()))
            .collect();
        result.dietary_tags = Some(sanitized);
    }

    Ok(result)
}

impl GeminiService {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_default();

        Self { api_key, client }
    }

    /// Calls the Gemini Interactions API to audit a recipe and extract missing metadata.
    pub async fn audit_recipe(&self, recipe: &Recipe) -> Result<AuditResult, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/interactions?key={}",
            self.api_key
        );

        let (prompt, response_schema) = build_prompt_and_schema(recipe);

        let payload = GeminiRequest {
            model: "models/gemini-3.5-flash-lite".to_string(),
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

        let raw_result: AuditResult = serde_json::from_str(&text_content).map_err(|e| {
            format!(
                "Failed to deserialize model output JSON schema: {}. Raw text: {}",
                e, text_content
            )
        })?;

        validate_and_sanitize_result(recipe, raw_result)
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

    fn make_full_recipe() -> Recipe {
        Recipe {
            id: ObjectId::new(),
            title: "Paella".to_string(),
            description: "Traditional Spanish dish".to_string(),
            categories: vec!["Rice".to_string()],
            ingredients: vec!["Rice".to_string(), "Saffron".to_string()],
            steps: vec!["Cook rice".to_string()],
            minutes: Some(45),
            method: Some("Frying pan".to_string()),
            calories: Some(600),
            recipe_cuisine: Some("Spanish".to_string()),
            recipe_yield: Some(4),
            gantt_table: Some(json!({})),
            difficulty: Some("Medium".to_string()),
            protein: Some(25),
            carbohydrates: Some(70),
            fat: Some(18),
            saturated_fat: Some(4),
            fiber: Some(3),
            sugar: Some(2),
            sodium: Some(750),
            dietary_tags: Some(vec!["gluten-free".to_string()]),
        }
    }

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
    fn test_audit_result_deserialization_with_new_fields() {
        let raw_json = r#"{
            "difficulty": "Easy",
            "protein": 24,
            "carbohydrates": 40,
            "fat": 12,
            "saturatedFat": 3,
            "fiber": 6,
            "sugar": 4,
            "sodium": 480,
            "dietaryTags": ["gluten-free", "high-protein"]
        }"#;
        let result: AuditResult = serde_json::from_str(raw_json).unwrap();
        assert_eq!(result.difficulty, Some("Easy".to_string()));
        assert_eq!(result.protein, Some(24));
        assert_eq!(result.carbohydrates, Some(40));
        assert_eq!(result.fat, Some(12));
        assert_eq!(result.saturated_fat, Some(3));
        assert_eq!(result.fiber, Some(6));
        assert_eq!(result.sugar, Some(4));
        assert_eq!(result.sodium, Some(480));
        assert_eq!(
            result.dietary_tags,
            Some(vec!["gluten-free".to_string(), "high-protein".to_string()])
        );
        assert_eq!(result.calories, None);
        assert_eq!(result.gantt_table, None);
    }

    #[test]
    fn test_build_prompt_and_schema_single_missing_field() {
        let mut recipe = make_full_recipe();
        recipe.recipe_cuisine = None;

        let (prompt, schema) = build_prompt_and_schema(&recipe);
        assert!(prompt.contains("1. Recipe cuisine (must be selected from the allowed enum list)."));
        assert!(!prompt.contains("1. Calories"));
        assert!(!prompt.contains("Difficulty"));
        assert!(!prompt.contains("Gantt Table"));

        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 1);
        assert_eq!(req_array[0], "recipeCuisine");
        assert!(schema["properties"].get("calories").is_none());
        assert!(schema["properties"].get("recipeCuisine").is_some());
    }

    #[test]
    fn test_build_prompt_and_schema_partial_reaudit_existing_recipes() {
        // Simulates one of the 600+ existing recipes that already has v1 fields
        let recipe = Recipe {
            id: ObjectId::new(),
            title: "Paella".to_string(),
            description: "Traditional Spanish dish".to_string(),
            categories: vec!["Rice".to_string()],
            ingredients: vec!["Rice".to_string(), "Saffron".to_string()],
            steps: vec!["Cook rice".to_string()],
            minutes: Some(50),
            method: Some("Frying pan".to_string()),
            calories: Some(600),
            recipe_cuisine: Some("Spanish".to_string()),
            recipe_yield: Some(4),
            gantt_table: Some(json!({"preSteps": [], "rows": [], "columns": []})),
            difficulty: None,
            protein: None,
            carbohydrates: None,
            fat: None,
            saturated_fat: None,
            fiber: None,
            sugar: None,
            sodium: None,
            dietary_tags: None,
        };

        let (prompt, schema) = build_prompt_and_schema(&recipe);
        // Should include existing metadata context for macro & difficulty consistency
        assert!(prompt.contains("Total Time: 50 minutes"));
        assert!(prompt.contains("Cooking Method: Frying pan"));
        assert!(prompt.contains("Existing Servings (yield): 4"));
        assert!(prompt.contains("Existing Calories per serving: 600 kcal"));
        assert!(prompt.contains("mathematically consistent"));

        // Should NOT re-request already populated fields
        assert!(schema["properties"].get("calories").is_none());
        assert!(schema["properties"].get("recipeCuisine").is_none());
        assert!(schema["properties"].get("recipeYield").is_none());
        assert!(schema["properties"].get("ganttTable").is_none());

        // Should request all 9 new fields
        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 9);
        for key in [
            "difficulty",
            "protein",
            "carbohydrates",
            "fat",
            "saturatedFat",
            "fiber",
            "sugar",
            "sodium",
            "dietaryTags",
        ] {
            assert!(
                schema["properties"].get(key).is_some(),
                "Expected schema property: {}",
                key
            );
        }
    }

    #[test]
    fn test_build_prompt_and_schema_all_missing_fields() {
        let recipe = Recipe {
            id: ObjectId::new(),
            title: "Tacos".to_string(),
            description: "Tasty tacos".to_string(),
            categories: vec![],
            ingredients: vec!["Tortilla".to_string()],
            steps: vec!["Assemble".to_string()],
            minutes: Some(15),
            method: Some("Frying pan".to_string()),
            calories: None,
            recipe_cuisine: None,
            recipe_yield: None,
            gantt_table: None,
            difficulty: None,
            protein: None,
            carbohydrates: None,
            fat: None,
            saturated_fat: None,
            fiber: None,
            sugar: None,
            sodium: None,
            dietary_tags: Some(vec![]), // simulates Prisma default on newly created recipe
        };

        let (_prompt, schema) = build_prompt_and_schema(&recipe);
        let req_array = schema["required"].as_array().unwrap();
        assert_eq!(req_array.len(), 13);
    }

    #[test]
    fn test_validate_and_sanitize_result_success_and_dedup() {
        let mut recipe = make_full_recipe();
        recipe.difficulty = None;
        recipe.dietary_tags = None;

        let raw = AuditResult {
            difficulty: Some("Easy".to_string()),
            dietary_tags: Some(vec![
                "vegan".to_string(),
                "gluten-free".to_string(),
                "vegan".to_string(),
                "invented-tag".to_string(),
            ]),
            ..Default::default()
        };

        let validated = validate_and_sanitize_result(&recipe, raw).unwrap();
        assert_eq!(validated.difficulty, Some("Easy".to_string()));
        assert_eq!(
            validated.dietary_tags,
            Some(vec!["vegan".to_string(), "gluten-free".to_string()])
        );
    }

    #[test]
    fn test_validate_and_sanitize_result_rejects_missing_or_invalid() {
        let mut recipe = make_full_recipe();
        recipe.difficulty = None;
        recipe.protein = None;

        // Missing protein
        let missing = AuditResult {
            difficulty: Some("Easy".to_string()),
            protein: None,
            ..Default::default()
        };
        assert!(validate_and_sanitize_result(&recipe, missing).is_err());

        // Invalid difficulty enum
        let invalid_diff = AuditResult {
            difficulty: Some("Impossible".to_string()),
            protein: Some(20),
            ..Default::default()
        };
        assert!(validate_and_sanitize_result(&recipe, invalid_diff).is_err());

        // Negative macro
        let negative = AuditResult {
            difficulty: Some("Easy".to_string()),
            protein: Some(-5),
            ..Default::default()
        };
        assert!(validate_and_sanitize_result(&recipe, negative).is_err());
    }
}
