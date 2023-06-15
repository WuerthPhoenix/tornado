use serde_json::Error;
use tornado_engine_api_dto::config::{
    ActionDto, ConstraintDto, ExtractorDto, ExtractorRegexDto, FilterDto,
    MatcherConfigDraftDataDto, MatcherConfigDraftDto, MatcherConfigDto, ModifierDto, OperatorDto,
    ProcessingTreeNodeEditDto, RuleDto,
};
use tornado_engine_matcher::config::filter::Filter;
use tornado_engine_matcher::config::rule::{
    ConfigAction, Constraint, Extractor, ExtractorRegex, Modifier, Operator, Rule,
};
use tornado_engine_matcher::config::{
    Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
};

pub fn matcher_config_draft_into_dto(
    draft: MatcherConfigDraft,
) -> Result<MatcherConfigDraftDto, Error> {
    Ok(MatcherConfigDraftDto {
        data: MatcherConfigDraftDataDto {
            user: draft.data.user,
            created_ts_ms: draft.data.created_ts_ms,
            updated_ts_ms: draft.data.updated_ts_ms,
            draft_id: draft.data.draft_id,
        },
        config: matcher_config_into_dto(draft.config)?,
    })
}

pub fn matcher_config_into_dto(config: MatcherConfig) -> Result<MatcherConfigDto, Error> {
    Ok(match config {
        MatcherConfig::Ruleset { name, rules } => MatcherConfigDto::Ruleset {
            name,
            rules: rules.into_iter().map(rule_into_dto).collect::<Result<Vec<_>, _>>()?,
        },
        MatcherConfig::Filter { name, filter, nodes } => MatcherConfigDto::Filter {
            name,
            filter: filter.into(),
            nodes: nodes.into_iter().map(matcher_config_into_dto).collect::<Result<Vec<_>, _>>()?,
        },
    })
}

pub fn rule_into_dto(rule: Rule) -> Result<RuleDto, Error> {
    Ok(RuleDto {
        active: rule.active,
        actions: rule.actions.into_iter().map(action_into_dto).collect::<Result<Vec<_>, _>>()?,
        constraint: constraint_into_dto(rule.constraint)?,
        description: rule.description,
        do_continue: rule.do_continue,
        name: rule.name,
    })
}

fn action_into_dto(action: ConfigAction) -> Result<ActionDto, Error> {
    Ok(ActionDto { id: action.id, payload: serde_json::to_value(action.payload)? })
}

fn constraint_into_dto(constraint: Constraint) -> Result<ConstraintDto, Error> {
    Ok(ConstraintDto {
        where_operator: constraint
            .where_operator
            .map(|operator_dto| OperatorDto::from(&operator_dto)),
        with: constraint
            .with
            .into_iter()
            .map(|(key, value)| (key, extractor_into_dto(value)))
            .collect(),
    })
}

fn extractor_into_dto(extractor: Extractor) -> ExtractorDto {
    ExtractorDto {
        from: extractor.from,
        regex: extractor_regex_into_dto(extractor.regex),
        modifiers_post: extractor
            .modifiers_post
            .into_iter()
            .map(|modifier| match modifier {
                Modifier::Lowercase {} => ModifierDto::Lowercase {},
                Modifier::Map { mapping, default_value } => {
                    ModifierDto::Map { mapping, default_value }
                }
                Modifier::ReplaceAll { find, replace, is_regex } => {
                    ModifierDto::ReplaceAll { find, replace, is_regex }
                }
                Modifier::ToNumber {} => ModifierDto::ToNumber {},
                Modifier::Trim {} => ModifierDto::Trim {},
            })
            .collect(),
    }
}

fn extractor_regex_into_dto(extractor_regex: ExtractorRegex) -> ExtractorRegexDto {
    match extractor_regex {
        ExtractorRegex::Regex { regex, all_matches, group_match_idx } => {
            ExtractorRegexDto::Regex { regex, all_matches, group_match_idx }
        }
        ExtractorRegex::RegexNamedGroups { regex, all_matches } => {
            ExtractorRegexDto::RegexNamedGroups { regex, all_matches }
        }
        ExtractorRegex::SingleKeyRegex { regex } => ExtractorRegexDto::KeyRegex { regex },
    }
}

pub fn dto_into_matcher_config_draft(
    draft: MatcherConfigDraftDto,
) -> Result<MatcherConfigDraft, Error> {
    Ok(MatcherConfigDraft {
        data: MatcherConfigDraftData {
            user: draft.data.user,
            created_ts_ms: draft.data.created_ts_ms,
            updated_ts_ms: draft.data.updated_ts_ms,
            draft_id: draft.data.draft_id,
        },
        config: dto_into_matcher_config(draft.config)?,
    })
}

pub fn dto_into_matcher_config(config: MatcherConfigDto) -> Result<MatcherConfig, Error> {
    Ok(match config {
        MatcherConfigDto::Ruleset { name, rules } => MatcherConfig::Ruleset {
            name,
            rules: rules.into_iter().map(dto_into_rule).collect::<Result<Vec<_>, _>>()?,
        },
        MatcherConfigDto::Filter { name, filter, nodes } => MatcherConfig::Filter {
            name,
            filter: dto_into_filter(filter)?,
            nodes: nodes.into_iter().map(dto_into_matcher_config).collect::<Result<Vec<_>, _>>()?,
        },
    })
}

pub fn processing_tree_node_details_dto_into_matcher_config(
    config: ProcessingTreeNodeEditDto,
) -> Result<MatcherConfig, Error> {
    Ok(match config {
        ProcessingTreeNodeEditDto::Ruleset { name } => {
            MatcherConfig::Ruleset { name, rules: vec![] }
        }
        ProcessingTreeNodeEditDto::Filter { name, description, active, filter } => {
            let filter_matcher_config = if let Some(filter_inner) = filter {
                Defaultable::from(Option::Some(dto_into_operator(filter_inner)?))
            } else {
                Defaultable::Default {}
            };
            MatcherConfig::Filter {
                name,
                filter: Filter { description, filter: filter_matcher_config, active },
                nodes: vec![],
            }
        }
    })
}

fn dto_into_filter(filter: FilterDto) -> Result<Filter, Error> {
    let option_filter = filter.filter.map(dto_into_operator).transpose()?;
    Ok(Filter {
        description: filter.description,
        filter: option_filter.into(),
        active: filter.active,
    })
}

pub fn dto_into_rule(rule: RuleDto) -> Result<Rule, Error> {
    Ok(Rule {
        active: rule.active,
        actions: rule.actions.into_iter().map(dto_into_action).collect::<Result<Vec<_>, _>>()?,
        constraint: dto_into_constraint(rule.constraint)?,
        description: rule.description,
        do_continue: rule.do_continue,
        name: rule.name,
    })
}

fn dto_into_action(action: ActionDto) -> Result<ConfigAction, Error> {
    Ok(ConfigAction { id: action.id, payload: serde_json::from_value(action.payload)? })
}

fn dto_into_constraint(constraint: ConstraintDto) -> Result<Constraint, Error> {
    Ok(Constraint {
        where_operator: constraint.where_operator.map(dto_into_operator).transpose()?,
        with: constraint
            .with
            .into_iter()
            .map(|(key, value)| (key, dto_into_extractor(value)))
            .collect(),
    })
}

fn dto_into_operator(operator: OperatorDto) -> Result<Operator, Error> {
    let result = match operator {
        OperatorDto::And { operators } => Operator::And {
            operators: operators
                .into_iter()
                .map(dto_into_operator)
                .collect::<Result<Vec<_>, _>>()?,
        },
        OperatorDto::Or { operators } => Operator::Or {
            operators: operators
                .into_iter()
                .map(dto_into_operator)
                .collect::<Result<Vec<_>, _>>()?,
        },
        OperatorDto::Not { operator } => {
            Operator::Not { operator: Box::new(dto_into_operator(*operator)?) }
        }
        OperatorDto::Contains { first, second } => Operator::Contains {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::ContainsIgnoreCase { first, second } => Operator::ContainsIgnoreCase {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::Equals { first, second } => Operator::Equals {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::EqualsIgnoreCase { first, second } => Operator::EqualsIgnoreCase {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::GreaterEqualThan { first, second } => Operator::GreaterEqualThan {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::GreaterThan { first, second } => Operator::GreaterThan {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::LessEqualThan { first, second } => Operator::LessEqualThan {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::LessThan { first, second } => Operator::LessThan {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::NotEquals { first, second } => Operator::NotEquals {
            first: serde_json::from_value(first)?,
            second: serde_json::from_value(second)?,
        },
        OperatorDto::Regex { regex, target } => Operator::Regex { regex, target },
    };
    Ok(result)
}

fn dto_into_extractor(extractor: ExtractorDto) -> Extractor {
    Extractor {
        from: extractor.from,
        regex: dto_into_extractor_regex(extractor.regex),
        modifiers_post: extractor
            .modifiers_post
            .into_iter()
            .map(|modifier| match modifier {
                ModifierDto::Lowercase {} => Modifier::Lowercase {},
                ModifierDto::Map { mapping, default_value } => {
                    Modifier::Map { mapping, default_value }
                }
                ModifierDto::ReplaceAll { find, replace, is_regex } => {
                    Modifier::ReplaceAll { find, replace, is_regex }
                }
                ModifierDto::ToNumber {} => Modifier::ToNumber {},
                ModifierDto::Trim {} => Modifier::Trim {},
            })
            .collect(),
    }
}

fn dto_into_extractor_regex(extractor_regex: ExtractorRegexDto) -> ExtractorRegex {
    match extractor_regex {
        ExtractorRegexDto::Regex { regex, all_matches, group_match_idx } => {
            ExtractorRegex::Regex { regex, all_matches, group_match_idx }
        }
        ExtractorRegexDto::RegexNamedGroups { regex, all_matches } => {
            ExtractorRegex::RegexNamedGroups { regex, all_matches }
        }
        ExtractorRegexDto::KeyRegex { regex } => ExtractorRegex::SingleKeyRegex { regex },
    }
}

#[cfg(test)]
mod test {
    use crate::config::convert::processing_tree_node_details_dto_into_matcher_config;
    use serde_json::json;
    use tornado_engine_api_dto::config::{OperatorDto, ProcessingTreeNodeEditDto};
    use tornado_engine_matcher::config::filter::Filter;
    use tornado_engine_matcher::config::rule::Operator;
    use tornado_engine_matcher::config::{Defaultable, MatcherConfig};

    #[actix_rt::test]
    async fn processing_tree_node_details_dto_filter_into_matcher_config_should_return_a_matcher_config_filter(
    ) {
        // Arrange
        let expected_maatcher_config_filter_with_empty_filter = MatcherConfig::Filter {
            name: "test_filter".to_string(),
            filter: Filter {
                description: "test_filter description".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };
        let expected_maatcher_config_filter = MatcherConfig::Filter {
            name: "test_filter".to_string(),
            filter: Filter {
                description: "test_filter description".to_string(),
                active: false,
                filter: Defaultable::from(Option::Some(Operator::And {
                    operators: vec![Operator::Equals { first: json!(12), second: json!(15) }],
                })),
            },
            nodes: vec![],
        };
        let processing_tree_node_details_dto_filter_with_empty_filter =
            ProcessingTreeNodeEditDto::Filter {
                name: "test_filter".to_string(),
                description: "test_filter description".to_string(),
                active: false,
                filter: None,
            };
        let processing_tree_node_details_dto = ProcessingTreeNodeEditDto::Filter {
            name: "test_filter".to_string(),
            description: "test_filter description".to_string(),
            active: false,
            filter: Option::Some(OperatorDto::And {
                operators: vec![OperatorDto::Equals { first: json!(12), second: json!(15) }],
            }),
        };

        // Act
        let actual_maatcher_config_filter_with_empty_filter =
            processing_tree_node_details_dto_into_matcher_config(
                processing_tree_node_details_dto_filter_with_empty_filter,
            );
        let actual_maatcher_config_filter =
            processing_tree_node_details_dto_into_matcher_config(processing_tree_node_details_dto);

        // Assert
        assert_eq!(
            actual_maatcher_config_filter_with_empty_filter.unwrap(),
            expected_maatcher_config_filter_with_empty_filter
        );
        assert_eq!(actual_maatcher_config_filter.unwrap(), expected_maatcher_config_filter);
    }

    #[actix_rt::test]
    async fn processing_tree_node_details_dto_ruleset_into_matcher_config_should_return_a_matcher_config_ruleset(
    ) {
        // Arrange
        let expected_maatcher_config_ruleset =
            MatcherConfig::Ruleset { name: "ruleset_test".to_string(), rules: vec![] };
        let processing_tree_node_details_dto_ruleset =
            ProcessingTreeNodeEditDto::Ruleset { name: "ruleset_test".to_string() };

        // Act
        let actual_maatcher_config_ruleset = processing_tree_node_details_dto_into_matcher_config(
            processing_tree_node_details_dto_ruleset,
        );

        // Assert
        assert_eq!(actual_maatcher_config_ruleset.unwrap(), expected_maatcher_config_ruleset);
    }
}
