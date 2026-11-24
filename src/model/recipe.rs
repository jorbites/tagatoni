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
    #[serde(default)]
    pub minutes: Option<i32>,
    #[serde(default)]
    pub method: Option<String>,
    pub calories: Option<i32>,
    #[serde(rename = "recipeCuisine")]
    pub recipe_cuisine: Option<String>,
    #[serde(rename = "recipeYield")]
    pub recipe_yield: Option<i32>,
    #[serde(rename = "ganttTable")]
    pub gantt_table: Option<serde_json::Value>,
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
    pub allergens: Option<Vec<String>>,
}

impl Recipe {
    /// Returns true if dietary tags have not yet been audited.
    ///
    /// Existing legacy documents omit `dietaryTags` (`None`). Newly created
    /// recipes written via Prisma initialize `dietaryTags` to `[]` while
    /// `difficulty` is still `None`, so an empty array alongside a missing
    /// `difficulty` also indicates an unaudited recipe.
    pub fn needs_dietary_tags(&self) -> bool {
        self.dietary_tags.is_none()
            || (self.dietary_tags.as_ref().is_some_and(|t| t.is_empty())
                && self.difficulty.is_none())
    }

    /// Returns true if EU-14 allergens have not yet been audited.
    pub fn needs_allergens(&self) -> bool {
        self.allergens.is_none()
            || (self.allergens.as_ref().is_some_and(|a| a.is_empty()) && self.difficulty.is_none())
    }

    /// Returns true if every Tagatoni-managed field is already populated.
    pub fn is_fully_audited(&self) -> bool {
        self.calories.is_some()
            && self.recipe_cuisine.is_some()
            && self.recipe_yield.is_some()
            && self.gantt_table.is_some()
            && self.difficulty.is_some()
            && self.protein.is_some()
            && self.carbohydrates.is_some()
            && self.fat.is_some()
            && self.saturated_fat.is_some()
            && self.fiber.is_some()
            && self.sugar.is_some()
            && self.sodium.is_some()
            && !self.needs_dietary_tags()
            && !self.needs_allergens()
    }
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
            "minutes": 25,
            "method": "Frying pan",
            "calories": 650,
            "recipeCuisine": "Italian",
            "recipeYield": 4,
            "difficulty": "Medium",
            "protein": 28,
            "carbohydrates": 72,
            "fat": 26,
            "saturatedFat": 10,
            "fiber": 3,
            "sugar": 2,
            "sodium": 820,
            "dietaryTags": ["comfort-food"],
            "allergens": ["gluten", "eggs", "milk"]
        });

        let recipe: Recipe = serde_json::from_value(recipe_json).unwrap();
        assert_eq!(recipe.title, "Spaghetti Carbonara");
        assert_eq!(recipe.id.to_hex(), "507f1f77bcf86cd799439011");
        assert_eq!(recipe.categories.len(), 2);
        assert_eq!(recipe.ingredients[0], "Spaghetti");
        assert_eq!(recipe.steps[2], "Mix egg yolk with cheese");
        assert_eq!(recipe.minutes, Some(25));
        assert_eq!(recipe.method, Some("Frying pan".to_string()));
        assert_eq!(recipe.calories, Some(650));
        assert_eq!(recipe.recipe_cuisine, Some("Italian".to_string()));
        assert_eq!(recipe.recipe_yield, Some(4));
        assert_eq!(recipe.difficulty, Some("Medium".to_string()));
        assert_eq!(recipe.protein, Some(28));
        assert_eq!(recipe.carbohydrates, Some(72));
        assert_eq!(recipe.fat, Some(26));
        assert_eq!(recipe.saturated_fat, Some(10));
        assert_eq!(recipe.fiber, Some(3));
        assert_eq!(recipe.sugar, Some(2));
        assert_eq!(recipe.sodium, Some(820));
        assert_eq!(recipe.dietary_tags, Some(vec!["comfort-food".to_string()]));
        assert_eq!(
            recipe.allergens,
            Some(vec![
                "gluten".to_string(),
                "eggs".to_string(),
                "milk".to_string()
            ])
        );
        assert!(!recipe.needs_dietary_tags());
        assert!(!recipe.needs_allergens());
        assert!(!recipe.is_fully_audited()); // missing ganttTable
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
        assert_eq!(recipe.difficulty, None);
        assert_eq!(recipe.protein, None);
        assert_eq!(recipe.carbohydrates, None);
        assert_eq!(recipe.fat, None);
        assert_eq!(recipe.saturated_fat, None);
        assert_eq!(recipe.fiber, None);
        assert_eq!(recipe.sugar, None);
        assert_eq!(recipe.sodium, None);
        assert_eq!(recipe.dietary_tags, None);
        assert_eq!(recipe.allergens, None);
        assert!(recipe.needs_dietary_tags());
        assert!(recipe.needs_allergens());
        assert!(!recipe.is_fully_audited());
    }

    #[test]
    fn test_needs_dietary_tags_distinguishes_prisma_default_from_audited_empty() {
        let unaudited_prisma_json = json!({
            "_id": { "$oid": "507f1f77bcf86cd799439011" },
            "title": "New Recipe",
            "description": "Just created",
            "categories": [],
            "ingredients": ["Flour"],
            "steps": ["Mix"],
            "difficulty": null,
            "dietaryTags": [],
            "allergens": []
        });
        let unaudited: Recipe = serde_json::from_value(unaudited_prisma_json).unwrap();
        assert!(unaudited.needs_dietary_tags());
        assert!(unaudited.needs_allergens());

        let audited_empty_json = json!({
            "_id": { "$oid": "507f1f77bcf86cd799439011" },
            "title": "Audited Recipe",
            "description": "No dietary tags or allergens apply",
            "categories": [],
            "ingredients": ["Rice", "Olive oil", "Salt"],
            "steps": ["Cook"],
            "calories": 500,
            "recipeCuisine": "Spanish",
            "recipeYield": 4,
            "ganttTable": {"preSteps": [], "rows": [], "columns": []},
            "difficulty": "Easy",
            "protein": 20,
            "carbohydrates": 45,
            "fat": 25,
            "saturatedFat": 9,
            "fiber": 2,
            "sugar": 3,
            "sodium": 600,
            "dietaryTags": [],
            "allergens": []
        });
        let audited: Recipe = serde_json::from_value(audited_empty_json).unwrap();
        assert!(!audited.needs_dietary_tags());
        assert!(!audited.needs_allergens());
        assert!(audited.is_fully_audited());
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
