use crate::model::recipe::Recipe;
use crate::service::gemini::AuditResult;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Client;

pub struct MongoService {
    client: Client,
    db_name: String,
}

pub fn build_set_document(result: &AuditResult) -> Result<mongodb::bson::Document, String> {
    let mut set_doc = doc! {};
    if let Some(cal) = result.calories {
        set_doc.insert("calories", cal);
    }
    if let Some(ref cuisine) = result.recipe_cuisine {
        set_doc.insert("recipeCuisine", cuisine.as_str());
    }
    if let Some(y) = result.recipe_yield {
        set_doc.insert("recipeYield", y);
    }
    if let Some(ref gt) = result.gantt_table {
        let bson_val = mongodb::bson::to_bson(gt)
            .map_err(|e| format!("Failed to convert ganttTable to BSON: {}", e))?;
        set_doc.insert("ganttTable", bson_val);
    }
    if let Some(ref difficulty) = result.difficulty {
        set_doc.insert("difficulty", difficulty.as_str());
    }
    if let Some(protein) = result.protein {
        set_doc.insert("protein", protein);
    }
    if let Some(carbohydrates) = result.carbohydrates {
        set_doc.insert("carbohydrates", carbohydrates);
    }
    if let Some(fat) = result.fat {
        set_doc.insert("fat", fat);
    }
    if let Some(saturated_fat) = result.saturated_fat {
        set_doc.insert("saturatedFat", saturated_fat);
    }
    if let Some(fiber) = result.fiber {
        set_doc.insert("fiber", fiber);
    }
    if let Some(sugar) = result.sugar {
        set_doc.insert("sugar", sugar);
    }
    if let Some(sodium) = result.sodium {
        set_doc.insert("sodium", sodium);
    }
    if let Some(ref dietary_tags) = result.dietary_tags {
        set_doc.insert("dietaryTags", dietary_tags);
    }
    Ok(set_doc)
}

pub fn build_missing_fields_filter(excluded_ids: &[ObjectId]) -> mongodb::bson::Document {
    let mut filter = doc! {
        "$or": [
            { "calories": { "$exists": false } },
            { "calories": null },
            { "recipeCuisine": { "$exists": false } },
            { "recipeCuisine": null },
            { "recipeYield": { "$exists": false } },
            { "recipeYield": null },
            { "ganttTable": { "$exists": false } },
            { "ganttTable": null },
            { "difficulty": { "$exists": false } },
            { "difficulty": null },
            { "protein": { "$exists": false } },
            { "protein": null },
            { "carbohydrates": { "$exists": false } },
            { "carbohydrates": null },
            { "fat": { "$exists": false } },
            { "fat": null },
            { "saturatedFat": { "$exists": false } },
            { "saturatedFat": null },
            { "fiber": { "$exists": false } },
            { "fiber": null },
            { "sugar": { "$exists": false } },
            { "sugar": null },
            { "sodium": { "$exists": false } },
            { "sodium": null },
            { "dietaryTags": { "$exists": false } },
            { "dietaryTags": null }
        ]
    };

    if !excluded_ids.is_empty() {
        filter.insert("_id", doc! { "$nin": excluded_ids });
    }

    filter
}

impl MongoService {
    pub fn new(client: Client, db_name: String) -> Self {
        Self { client, db_name }
    }

    /// Fetches a single recipe that has any missing Tagatoni-managed metadata field,
    /// excluding any recipe IDs currently blocked in Turso.
    pub async fn fetch_next_recipe_to_audit(
        &self,
        excluded_ids: &[ObjectId],
    ) -> Result<Option<Recipe>, mongodb::error::Error> {
        let recipe_collection = self
            .client
            .database(&self.db_name)
            .collection::<Recipe>("Recipe");

        let filter = build_missing_fields_filter(excluded_ids);

        recipe_collection.find_one(filter).await
    }

    pub async fn update_recipe_seo(
        &self,
        id: &ObjectId,
        result: &AuditResult,
    ) -> Result<(), String> {
        let set_doc = build_set_document(result)?;
        if set_doc.is_empty() {
            return Ok(());
        }

        let recipe_collection = self
            .client
            .database(&self.db_name)
            .collection::<mongodb::bson::Document>("Recipe");

        let filter = doc! { "_id": id };
        let update = doc! { "$set": set_doc };

        recipe_collection
            .update_one(filter, update)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_set_document_all_fields() {
        let result = AuditResult {
            calories: Some(500),
            recipe_cuisine: Some("Italian".to_string()),
            recipe_yield: Some(4),
            gantt_table: Some(json!({"preSteps": [], "rows": [], "columns": []})),
            difficulty: Some("Medium".to_string()),
            protein: Some(22),
            carbohydrates: Some(60),
            fat: Some(18),
            saturated_fat: Some(5),
            fiber: Some(4),
            sugar: Some(3),
            sodium: Some(650),
            dietary_tags: Some(vec!["vegetarian".to_string(), "high-protein".to_string()]),
        };

        let doc = build_set_document(&result).unwrap();
        assert_eq!(doc.get_i32("calories").unwrap(), 500);
        assert_eq!(doc.get_str("recipeCuisine").unwrap(), "Italian");
        assert_eq!(doc.get_i32("recipeYield").unwrap(), 4);
        assert!(doc.get("ganttTable").is_some());
        assert_eq!(doc.get_str("difficulty").unwrap(), "Medium");
        assert_eq!(doc.get_i32("protein").unwrap(), 22);
        assert_eq!(doc.get_i32("carbohydrates").unwrap(), 60);
        assert_eq!(doc.get_i32("fat").unwrap(), 18);
        assert_eq!(doc.get_i32("saturatedFat").unwrap(), 5);
        assert_eq!(doc.get_i32("fiber").unwrap(), 4);
        assert_eq!(doc.get_i32("sugar").unwrap(), 3);
        assert_eq!(doc.get_i32("sodium").unwrap(), 650);
        assert_eq!(doc.get_array("dietaryTags").unwrap().len(), 2);
        assert_eq!(doc.len(), 13);
    }

    #[test]
    fn test_build_set_document_partial_new_fields_with_empty_dietary_tags() {
        let result = AuditResult {
            difficulty: Some("Easy".to_string()),
            protein: Some(15),
            carbohydrates: Some(30),
            fat: Some(10),
            saturated_fat: Some(2),
            fiber: Some(0),
            sugar: Some(0),
            sodium: Some(320),
            dietary_tags: Some(vec![]),
            ..Default::default()
        };

        let doc = build_set_document(&result).unwrap();
        assert!(doc.get("calories").is_none());
        assert!(doc.get("recipeCuisine").is_none());
        assert!(doc.get("recipeYield").is_none());
        assert!(doc.get("ganttTable").is_none());
        assert_eq!(doc.get_str("difficulty").unwrap(), "Easy");
        assert_eq!(doc.get_i32("fiber").unwrap(), 0);
        assert_eq!(doc.get_i32("sugar").unwrap(), 0);
        assert!(doc.get_array("dietaryTags").unwrap().is_empty());
        assert_eq!(doc.len(), 9);
    }

    #[test]
    fn test_build_set_document_empty() {
        let doc = build_set_document(&AuditResult::default()).unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_build_missing_fields_filter_with_and_without_exclusions() {
        let filter_no_exclusions = build_missing_fields_filter(&[]);
        assert!(filter_no_exclusions.get("_id").is_none());
        assert!(filter_no_exclusions.get_array("$or").unwrap().len() >= 26);

        let oid = ObjectId::new();
        let filter_with_exclusions = build_missing_fields_filter(&[oid]);
        let id_filter = filter_with_exclusions.get_document("_id").unwrap();
        assert_eq!(id_filter.get_array("$nin").unwrap().len(), 1);
    }
}
