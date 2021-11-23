use serde_json::Error;
use tornado_engine_api_dto::config::{ActionDto, ConstraintDto, ExtractorDto, ExtractorRegexDto, ExtractorRegexTypeDto, ExtractorTextDto, FilterDto, MatcherConfigDraftDataDto, MatcherConfigDraftDto, MatcherConfigDto, ModifierDto, OperatorDto, RuleDto};
use tornado_engine_matcher::config::filter::Filter;
use tornado_engine_matcher::config::rule::{Action, Constraint, Extractor, ExtractorRegex, ExtractorRegexType, ExtractorText, Modifier, Operator, Rule};
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData};

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

fn action_into_dto(action: Action) -> Result<ActionDto, Error> {
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
    match extractor {
        Extractor::Regex(extractor) => {
            ExtractorDto::Regex(ExtractorRegexDto {
                from: extractor.from,
                regex: extractor_regex_into_dto(extractor.regex),
                modifiers_post: modifiers_post_into_dto(extractor.modifiers_post),
            })
        },
        Extractor::Text(extractor) => {
            ExtractorDto::Text(ExtractorTextDto { 
                text: extractor.text,
                modifiers_post: modifiers_post_into_dto(extractor.modifiers_post),
            })
        }
    }
}

fn modifiers_post_into_dto(modifiers_post: Vec<Modifier>) -> Vec<ModifierDto> {
    modifiers_post
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
    .collect()
}

fn extractor_regex_into_dto(extractor_regex: ExtractorRegexType) -> ExtractorRegexTypeDto {
    match extractor_regex {
        ExtractorRegexType::Regex { regex, all_matches, group_match_idx } => {
            ExtractorRegexTypeDto::Regex { regex, all_matches, group_match_idx }
        }
        ExtractorRegexType::RegexNamedGroups { regex, all_matches } => {
            ExtractorRegexTypeDto::RegexNamedGroups { regex, all_matches }
        }
        ExtractorRegexType::SingleKeyRegex { regex } => ExtractorRegexTypeDto::KeyRegex { regex },
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

fn dto_into_filter(filter: FilterDto) -> Result<Filter, Error> {
    let option_filter = filter.filter.map(dto_into_operator).transpose()?;
    Ok(Filter {
        description: filter.description,
        filter: option_filter.into(),
        active: filter.active,
    })
}

fn dto_into_rule(rule: RuleDto) -> Result<Rule, Error> {
    Ok(Rule {
        active: rule.active,
        actions: rule.actions.into_iter().map(dto_into_action).collect::<Result<Vec<_>, _>>()?,
        constraint: dto_into_constraint(rule.constraint)?,
        description: rule.description,
        do_continue: rule.do_continue,
        name: rule.name,
    })
}

fn dto_into_action(action: ActionDto) -> Result<Action, Error> {
    Ok(Action { id: action.id, payload: serde_json::from_value(action.payload)? })
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
    match extractor {
        ExtractorDto::Regex(extractor) => {
            Extractor::Regex(ExtractorRegex {
                from: extractor.from,
                regex: dto_into_extractor_regex(extractor.regex),
                modifiers_post: dto_into_modifiers_post(extractor.modifiers_post),
            })
        },
        ExtractorDto::Text(extractor) => {
            Extractor::Text(ExtractorText  {
                text: extractor.text,
                modifiers_post: dto_into_modifiers_post(extractor.modifiers_post),
            })
        }
    }
}

fn dto_into_modifiers_post(modifiers_post: Vec<ModifierDto>) -> Vec<Modifier> {
    modifiers_post
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
        .collect()
}

fn dto_into_extractor_regex(extractor_regex: ExtractorRegexTypeDto) -> ExtractorRegexType {
    match extractor_regex {
        ExtractorRegexTypeDto::Regex { regex, all_matches, group_match_idx } => {
            ExtractorRegexType::Regex { regex, all_matches, group_match_idx }
        }
        ExtractorRegexTypeDto::RegexNamedGroups { regex, all_matches } => {
            ExtractorRegexType::RegexNamedGroups { regex, all_matches }
        }
        ExtractorRegexTypeDto::KeyRegex { regex } => ExtractorRegexType::SingleKeyRegex { regex },
    }
}
