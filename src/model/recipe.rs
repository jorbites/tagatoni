use mongodb::bson::oid::ObjectId;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
    pub calories: Option<i32>,
    #[serde(rename = "recipeCuisine")]
    pub recipe_cuisine: Option<String>,
    #[serde(rename = "recipeYield")]
    pub recipe_yield: Option<i32>,
    #[serde(rename = "ganttTable")]
    pub gantt_table: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_recipe() {
        let recipe_json = json!({
            "_id": { "$oid": "507f1f77bcf86cd799439011" },
            "title": "Spaghetti Carbonara",
            "description": "Classic Roman pasta dish",
            "categories": ["Pasta", "Italian"],
            "ingredients": ["Spaghetti", "Pancetta", "Eggs", "Pecorino Romano"],
            "steps": ["Boil pasta", "Fry pancetta", "Mix egg yolk with cheese", "Combine and toss"],
            "calories": 650,
            "recipeCuisine": "Italian",
            "recipeYield": 4
        });

        let recipe: Recipe = serde_json::from_value(recipe_json).unwrap();
        assert_eq!(recipe.title, "Spaghetti Carbonara");
        assert_eq!(recipe.id.to_hex(), "507f1f77bcf86cd799439011");
        assert_eq!(recipe.categories.len(), 2);
        assert_eq!(recipe.ingredients[0], "Spaghetti");
        assert_eq!(recipe.steps[2], "Mix egg yolk with cheese");
        assert_eq!(recipe.calories, Some(650));
        assert_eq!(recipe.recipe_cuisine, Some("Italian".to_string()));
        assert_eq!(recipe.recipe_yield, Some(4));
    }

    #[test]
    fn test_deserialize_recipe_missing_optional_fields() {
        let recipe_json = json!({
            "_id": { "$oid": "507f1f77bcf86cd799439011" },
            "title": "Basic Salad",
            "description": "Simple green salad",
            "categories": [],
            "ingredients": ["Lettuce", "Olive Oil"],
            "steps": ["Wash lettuce", "Drizzle oil"],
            "calories": null,
            "recipeCuisine": null,
            "recipeYield": null
        });

        let recipe: Recipe = serde_json::from_value(recipe_json).unwrap();
        assert_eq!(recipe.calories, None);
        assert_eq!(recipe.recipe_cuisine, None);
        assert_eq!(recipe.recipe_yield, None);
        assert_eq!(recipe.gantt_table, None);
    }

    #[test]
    fn test_deserialize_recipe_with_gantt_table() {
        let recipe_json = json!({
            "_id": { "$oid": "507f1f77bcf86cd799439011" },
            "title": "Brownies",
            "description": "Chocolate brownies",
            "categories": ["Dessert"],
            "ingredients": ["Butter", "Sugar"],
            "steps": ["Melt butter", "Mix"],
            "calories": 400,
            "recipeCuisine": "American",
            "recipeYield": 8,
            "ganttTable": {
                "preSteps": ["Preheat oven"],
                "rows": [{"ingredient": "Butter", "group": 0}],
                "columns": [{"action": "melt", "rowSpan": [0, 0], "colIndex": 0}]
            }
        });

        let recipe: Recipe = serde_json::from_value(recipe_json).unwrap();
        assert!(recipe.gantt_table.is_some());
        let table = recipe.gantt_table.unwrap();
        assert_eq!(table["preSteps"][0], "Preheat oven");
    }
}
