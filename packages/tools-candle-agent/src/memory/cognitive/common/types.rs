//! Common cognitive types
//!
//! Shared types for local cognitive operations.

use serde::{Deserialize, Serialize};

/// Committee configuration for local evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeConfig {
    pub members: Vec<String>,
    pub consensus_threshold: f64,
    pub evaluation_rounds: u32,
}

impl Default for CommitteeConfig {
    fn default() -> Self {
        Self {
            members: vec!["evaluator1".to_string(), "evaluator2".to_string()],
            consensus_threshold: 0.7,
            evaluation_rounds: 1,
        }
    }
}

/// Evaluation rubric for quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRubric {
    pub criteria: Vec<EvaluationCriterion>,
    pub scoring_method: ScoringMethod,
}

impl EvaluationRubric {
    pub fn from_spec(spec: &str) -> Result<Self, String> {
        if spec.trim().is_empty() {
            return Err("Specification cannot be empty".to_string());
        }

        // Try parsing as JSON first
        if let Ok(json_criteria) = Self::parse_json_spec(spec) {
            let scoring_method = if json_criteria.iter().any(|c| c.weight != 1.0) {
                ScoringMethod::Weighted
            } else {
                ScoringMethod::Average
            };
            return Ok(Self {
                criteria: json_criteria,
                scoring_method,
            });
        }

        // Fall back to simple delimited format: "name:weight:description;name:weight:description"
        if let Ok(simple_criteria) = Self::parse_simple_spec(spec) {
            let scoring_method = if simple_criteria.iter().any(|c| c.weight != 1.0) {
                ScoringMethod::Weighted
            } else {
                ScoringMethod::Average
            };
            return Ok(Self {
                criteria: simple_criteria,
                scoring_method,
            });
        }

        Err(format!("Invalid specification format: {}", spec))
    }

    fn parse_json_spec(spec: &str) -> Result<Vec<EvaluationCriterion>, String> {
        #[derive(Deserialize)]
        struct JsonCriterion {
            name: String,
            weight: Option<f64>,
            description: Option<String>,
        }

        let json_criteria: Vec<JsonCriterion> =
            serde_json::from_str(spec).map_err(|e| format!("JSON parsing error: {}", e))?;

        if json_criteria.is_empty() {
            return Err("Specification must contain at least one criterion".to_string());
        }

        let criteria = json_criteria
            .into_iter()
            .map(|jc| EvaluationCriterion {
                name: jc.name,
                weight: jc.weight.unwrap_or(1.0),
                description: jc
                    .description
                    .unwrap_or_else(|| "No description provided".to_string()),
            })
            .collect();

        Ok(criteria)
    }

    fn parse_simple_spec(spec: &str) -> Result<Vec<EvaluationCriterion>, String> {
        let mut criteria = Vec::new();

        for criterion_spec in spec.split(';') {
            let parts: Vec<&str> = criterion_spec.trim().split(':').collect();

            if parts.is_empty() || parts.len() > 3 {
                return Err(format!(
                    "Invalid criterion format: '{}'. Expected 'name[:weight[:description]]'",
                    criterion_spec
                ));
            }

            let name = parts[0].trim().to_string();
            if name.is_empty() {
                return Err("Criterion name cannot be empty".to_string());
            }

            let weight = if parts.len() > 1 {
                parts[1]
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid weight '{}' in criterion '{}'", parts[1], name))?
            } else {
                1.0
            };

            if !(0.0..=10.0).contains(&weight) {
                return Err(format!(
                    "Weight {} for criterion '{}' must be between 0.0 and 10.0",
                    weight, name
                ));
            }

            let description = if parts.len() > 2 {
                parts[2].trim().to_string()
            } else {
                format!("Evaluation criterion: {}", name)
            };

            criteria.push(EvaluationCriterion {
                name,
                weight,
                description,
            });
        }

        if criteria.is_empty() {
            return Err("Specification must contain at least one criterion".to_string());
        }

        Ok(criteria)
    }
}
/// Evaluation criterion for assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriterion {
    pub name: String,
    pub weight: f64,
    pub description: String,
}

impl Default for EvaluationCriterion {
    fn default() -> Self {
        Self {
            name: "quality".to_string(),
            weight: 1.0,
            description: "Overall content quality".to_string(),
        }
    }
}

/// Scoring method for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoringMethod {
    Average,
    Weighted,
    Consensus,
}

/// Impact factors for cognitive evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactFactors {
    pub relevance: f64,
    pub freshness: f64,
    pub quality: f64,
    pub complexity: f64,
}

impl Default for ImpactFactors {
    fn default() -> Self {
        Self {
            relevance: 1.0,
            freshness: 0.8,
            quality: 1.0,
            complexity: 0.6,
        }
    }
}

impl From<Vec<f64>> for ImpactFactors {
    fn from(factors: Vec<f64>) -> Self {
        Self {
            relevance: factors.first().copied().unwrap_or(1.0),
            freshness: factors.get(1).copied().unwrap_or(0.8),
            quality: factors.get(2).copied().unwrap_or(1.0),
            complexity: factors.get(3).copied().unwrap_or(0.6),
        }
    }
}
