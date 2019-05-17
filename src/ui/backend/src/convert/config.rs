use dto::config::{
    ActionDto, ConstraintDto, ExtractorDto, ExtractorRegexDto, FilterDto, MatcherConfigDto,
    OperatorDto, RuleDto,
};
use serde_json::Error;
use std::collections::btree_map::BTreeMap;
use tornado_engine_matcher::config::filter::Filter;
use tornado_engine_matcher::config::rule::{
    Action, Constraint, Extractor, ExtractorRegex, Operator, Rule,
};
use tornado_engine_matcher::config::MatcherConfig;

pub fn matcher_config_into_dto(config: MatcherConfig) -> Result<MatcherConfigDto, Error> {
    Ok(match config {
        MatcherConfig::Rules { rules } => MatcherConfigDto::Rules {
            rules: rules.into_iter().map(rule_into_dto).collect::<Result<Vec<_>, _>>()?,
        },
        MatcherConfig::Filter { filter, nodes } => MatcherConfigDto::Filter {
            filter: filter_into_dto(filter),
            nodes: nodes
                .into_iter()
                .map(|(key, value)| {
                    let dto = matcher_config_into_dto(value)?;
                    Ok((key, dto))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?,
        },
    })
}

pub fn filter_into_dto(filter: Filter) -> FilterDto {
    FilterDto {
        name: filter.name,
        description: filter.description,
        filter: filter.filter.map(operator_into_dto),
        active: filter.active,
    }
}

pub fn rule_into_dto(rule: Rule) -> Result<RuleDto, Error> {
    Ok(RuleDto {
        active: rule.active,
        actions: rule.actions.into_iter().map(action_into_dto).collect::<Result<Vec<_>, _>>()?,
        constraint: constraint_into_dto(rule.constraint),
        description: rule.description,
        do_continue: rule.do_continue,
        name: rule.name,
    })
}

pub fn action_into_dto(action: Action) -> Result<ActionDto, Error> {
    Ok(ActionDto { id: action.id, payload: serde_json::to_value(action.payload)? })
}

pub fn constraint_into_dto(constraint: Constraint) -> ConstraintDto {
    ConstraintDto {
        where_operator: constraint.where_operator.map(operator_into_dto),
        with: constraint
            .with
            .into_iter()
            .map(|(key, value)| (key, extractor_into_dto(value)))
            .collect(),
    }
}

pub fn operator_into_dto(operator: Operator) -> OperatorDto {
    match operator {
        Operator::And { operators } => {
            OperatorDto::And { operators: operators.into_iter().map(operator_into_dto).collect() }
        }
        Operator::Or { operators } => {
            OperatorDto::Or { operators: operators.into_iter().map(operator_into_dto).collect() }
        }
        Operator::Contain { text, substring } => OperatorDto::Contain { text, substring },
        Operator::Equal { first, second } => OperatorDto::Equal { first, second },
        Operator::Regex { regex, target } => OperatorDto::Regex { regex, target },
    }
}

pub fn extractor_into_dto(extractor: Extractor) -> ExtractorDto {
    ExtractorDto { from: extractor.from, regex: extractor_regex_into_dto(extractor.regex) }
}

pub fn extractor_regex_into_dto(extractor_regex: ExtractorRegex) -> ExtractorRegexDto {
    ExtractorRegexDto {
        group_match_idx: extractor_regex.group_match_idx,
        regex: extractor_regex.regex,
    }
}
