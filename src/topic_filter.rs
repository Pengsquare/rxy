use crate::feed::Paper;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TopicFilter
{
    #[serde(default)]
    pub categories: FilterCategories,
    #[serde(default)]
    pub keyword_rules: HashMap<String, Vec<KeywordRule>>,
    #[serde(default)]
    pub author_rules: HashMap<String, Vec<AuthorRule>>,
    #[serde(default)]
    pub exclusions: Exclusions,
}

#[derive(Debug, Default, Deserialize)]
pub struct FilterCategories
{
    #[serde(default)]
    pub primary: Vec<CategoryRule>,
    #[serde(default)]
    pub secondary: Vec<CategoryRule>,
    #[serde(default)]
    pub tertiary: Vec<CategoryRule>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryRule
{
    pub code: String,
    pub weight: i32,
}

#[derive(Debug, Deserialize)]
pub struct KeywordRule
{
    pub terms: Vec<String>,
    pub weight: i32,
}

#[derive(Debug, Deserialize)]
pub struct AuthorRule
{
    pub name: String,
    pub weight: i32,
}

#[derive(Debug, Default, Deserialize)]
pub struct Exclusions
{
    #[serde(default)]
    pub anti_keywords: Vec<KeywordRule>,
}

impl TopicFilter
{
    pub fn load(path: &str) -> Result<Self, String>
    {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read filter '{}': {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Cannot parse filter '{}': {}", path, e))
    }

    pub fn score(&self, paper: &Paper) -> i32
    {
        let mut score = 0i32;

        // Category weights: add weight for each matching category code.
        for rule in self
            .categories
            .primary
            .iter()
            .chain(self.categories.secondary.iter())
            .chain(self.categories.tertiary.iter())
        {
            if paper
                .categories
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&rule.code))
            {
                score += rule.weight;
            }
        }

        let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();

        // Keyword rules: any matching term in title+abstract adds the rule's weight.
        for rules in self.keyword_rules.values()
        {
            for rule in rules
            {
                if rule
                    .terms
                    .iter()
                    .any(|t| text.contains(t.to_lowercase().as_str()))
                {
                    score += rule.weight;
                }
            }
        }

        // Anti-keywords: weights are negative and push the score down.
        for rule in &self.exclusions.anti_keywords
        {
            if rule
                .terms
                .iter()
                .any(|t| text.contains(t.to_lowercase().as_str()))
            {
                score += rule.weight;
            }
        }

        // Author rules: substring match on author name (case-insensitive).
        for rules in self.author_rules.values()
        {
            for rule in rules
            {
                let name_lower = rule.name.to_lowercase();
                if paper
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&name_lower))
                {
                    score += rule.weight;
                }
            }
        }

        score
    }
}
