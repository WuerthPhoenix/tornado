use crate::event::api::SendEventRequest;
use serde_json::Error;
use std::collections::HashMap;
use tornado_common_api::Action;
use tornado_engine_api_dto::config::ActionDto;
use tornado_engine_api_dto::event::{
    ProcessType, ProcessedEventDto, ProcessedFilterDto, ProcessedFilterStatusDto,
    ProcessedIterationDto, ProcessedIteratorDto, ProcessedIteratorStatusDto, ProcessedNodeDto,
    ProcessedRuleDto, ProcessedRuleStatusDto, ProcessedRulesDto, ScopeVariableDto,
    SendEventRequestDto,
};
use tornado_engine_matcher::model::{
    ProcessedEvent, ProcessedFilter, ProcessedFilterStatus, ProcessedIteration, ProcessedIterator,
    ProcessedNode, ProcessedRule, ProcessedRuleStatus, ProcessedRules, ScopeVariable,
};

pub fn dto_into_send_event_request(dto: SendEventRequestDto) -> Result<SendEventRequest, Error> {
    Ok(SendEventRequest {
        process_type: match dto.process_type {
            ProcessType::Full => crate::event::api::ProcessType::Full,
            ProcessType::SkipActions => crate::event::api::ProcessType::SkipActions,
        },
        event: serde_json::from_value(serde_json::to_value(dto.event)?)?,
    })
}

pub fn processed_event_into_dto(
    processed_event: ProcessedEvent,
) -> Result<ProcessedEventDto, Error> {
    Ok(ProcessedEventDto {
        event: serde_json::from_value(processed_event.event)?,
        result: processed_node_into_dto(processed_event.result)?,
    })
}

pub fn processed_node_into_dto(node: ProcessedNode) -> Result<ProcessedNodeDto, Error> {
    Ok(match node {
        ProcessedNode::Ruleset { name, rules } => {
            ProcessedNodeDto::Ruleset { name, rules: processed_rules_into_dto(rules)? }
        }
        ProcessedNode::Filter { name, filter, nodes } => ProcessedNodeDto::Filter {
            name,
            nodes: nodes.into_iter().map(processed_node_into_dto).collect::<Result<Vec<_>, _>>()?,
            filter: processed_filter_into_dto(filter),
        },
        ProcessedNode::Iterator { name, iterator, events } => ProcessedNodeDto::Iterator {
            name,
            iterator: processed_iterator_into_dto(iterator),
            events: events
                .into_iter()
                .map(processed_iteration_into_dto)
                .collect::<Result<Vec<_>, _>>()?,
        },
    })
}

pub fn processed_iteration_into_dto(
    iteration: ProcessedIteration,
) -> Result<ProcessedIterationDto, Error> {
    Ok(ProcessedIterationDto {
        event: serde_json::from_value(iteration.event)?,
        nodes: iteration
            .result
            .into_iter()
            .map(processed_node_into_dto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn processed_rules_into_dto(node: ProcessedRules) -> Result<ProcessedRulesDto, Error> {
    Ok(ProcessedRulesDto {
        extracted_vars: serde_json::to_value(node.extracted_vars)?,
        rules: node
            .rules
            .into_iter()
            .map(processed_rule_into_dto)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn processed_rule_into_dto(node: ProcessedRule) -> Result<ProcessedRuleDto, Error> {
    Ok(ProcessedRuleDto {
        message: node.message,
        name: node.name,
        actions: node.actions.into_iter().map(action_into_dto).collect::<Result<Vec<_>, _>>()?,
        ruleset_scope_state: process_scope_states(node.ruleset_scope_state),
        status: processed_rule_status_into_dto(node.status),
        meta: node.meta,
    })
}

pub fn processed_rule_status_into_dto(node: ProcessedRuleStatus) -> ProcessedRuleStatusDto {
    match node {
        ProcessedRuleStatus::NotProcessed => ProcessedRuleStatusDto::NotProcessed,
        ProcessedRuleStatus::NotMatched => ProcessedRuleStatusDto::NotMatched,
        ProcessedRuleStatus::Matched => ProcessedRuleStatusDto::Matched,
        ProcessedRuleStatus::PartiallyMatched => ProcessedRuleStatusDto::PartiallyMatched,
    }
}

pub fn process_scope_states(
    scope_variable: Option<HashMap<String, ScopeVariable>>,
) -> Option<HashMap<String, ScopeVariableDto>> {
    Some(
        scope_variable?.into_iter().map(|(key, value)| (key, process_scope_state(value))).collect(),
    )
}

pub fn process_scope_state(scope_variable: ScopeVariable) -> ScopeVariableDto {
    ScopeVariableDto { source: scope_variable.source, value: scope_variable.value }
}

pub fn action_into_dto(action: Action) -> Result<ActionDto, Error> {
    Ok(ActionDto { id: action.id, payload: serde_json::to_value(action.payload)? })
}

pub fn processed_filter_into_dto(node: ProcessedFilter) -> ProcessedFilterDto {
    ProcessedFilterDto { status: processed_filter_status_into_dto(node.status) }
}

pub fn processed_iterator_into_dto(node: ProcessedIterator) -> ProcessedIteratorDto {
    match node {
        ProcessedIterator::Matched => {
            ProcessedIteratorDto { status: ProcessedIteratorStatusDto::Matched }
        }
        ProcessedIterator::AccessorError => {
            ProcessedIteratorDto { status: ProcessedIteratorStatusDto::AccessorError }
        }

        ProcessedIterator::TypeError => {
            ProcessedIteratorDto { status: ProcessedIteratorStatusDto::TypeError }
        }
    }
}

pub fn processed_filter_status_into_dto(node: ProcessedFilterStatus) -> ProcessedFilterStatusDto {
    match node {
        ProcessedFilterStatus::NotMatched => ProcessedFilterStatusDto::NotMatched,
        ProcessedFilterStatus::Matched => ProcessedFilterStatusDto::Matched,
        ProcessedFilterStatus::Inactive => ProcessedFilterStatusDto::Inactive,
    }
}
