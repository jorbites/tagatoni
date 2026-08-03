use crate::model::recipe::Recipe;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Client;

pub struct MongoService {
    client: Client,
    db_name: String,
}

pub fn build_set_document(
    calories: Option<i32>,
    recipe_cuisine: Option<&str>,
    recipe_yield: Option<i32>,
) -> mongodb::bson::Document {
    let mut set_doc = doc! {};
    if let Some(cal) = calories {
        set_doc.insert("calories", cal);
    }
    if let Some(cuisine) = recipe_cuisine {
        set_doc.insert("recipeCuisine", cuisine);
    }
    if let Some(y) = recipe_yield {
        set_doc.insert("recipeYield", y);
    }
    set_doc
}

impl MongoService {
    pub fn new(client: Client, db_name: String) -> Self {
        Self { client, db_name }
    }

    /// Fetches a single recipe that has missing calories, missing recipeCuisine, or missing recipeYield.
    /// We search for documents where calories is null or doesn't exist, OR recipeCuisine is null or doesn't exist, OR recipeYield is null or doesn't exist.
    pub async fn fetch_next_recipe_to_audit(
        &self,
    ) -> Result<Option<Recipe>, mongodb::error::Error> {
        let recipe_collection = self
            .client
            .database(&self.db_name)
            .collection::<Recipe>("Recipe");

        let filter = doc! {
            "$or": [
                { "calories": { "$exists": false } },
                { "calories": null },
                { "recipeCuisine": { "$exists": false } },
                { "recipeCuisine": null },
                { "recipeYield": { "$exists": false } },
                { "recipeYield": null }
            ]
        };

        recipe_collection.find_one(filter).await
    }

    pub async fn update_recipe_seo(
        &self,
        id: &ObjectId,
        calories: Option<i32>,
        recipe_cuisine: Option<&str>,
        recipe_yield: Option<i32>,
    ) -> Result<(), mongodb::error::Error> {
        let set_doc = build_set_document(calories, recipe_cuisine, recipe_yield);
        if set_doc.is_empty() {
            return Ok(());
        }

        let recipe_collection = self
            .client
            .database(&self.db_name)
            .collection::<mongodb::bson::Document>("Recipe");

        let filter = doc! { "_id": id };
        let update = doc! { "$set": set_doc };

        recipe_collection.update_one(filter, update).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_set_document_all_fields() {
        let doc = build_set_document(Some(500), Some("Italian"), Some(4));
        assert_eq!(doc.get_i32("calories").unwrap(), 500);
        assert_eq!(doc.get_str("recipeCuisine").unwrap(), "Italian");
        assert_eq!(doc.get_i32("recipeYield").unwrap(), 4);
        assert_eq!(doc.len(), 3);
    }

    #[test]
    fn test_build_set_document_partial_fields() {
        let doc = build_set_document(None, Some("Mexican"), None);
        assert!(doc.get("calories").is_none());
        assert_eq!(doc.get_str("recipeCuisine").unwrap(), "Mexican");
        assert!(doc.get("recipeYield").is_none());
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_build_set_document_empty() {
        let doc = build_set_document(None, None, None);
        assert!(doc.is_empty());
    }
}
