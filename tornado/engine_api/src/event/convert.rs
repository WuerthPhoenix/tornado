use crate::event::api::SendEventRequest;
use serde_json::Error;
use tornado_common_api::Action;
use tornado_engine_api_dto::config::ActionDto;
use tornado_engine_api_dto::event::{
    ProcessType, ProcessedEventDto, ProcessedFilterDto, ProcessedFilterStatusDto,
    ProcessedIteratorDto, ProcessedIteratorStatusDto, ProcessedNodeDto, ProcessedRuleDto,
    ProcessedRuleStatusDto, ProcessedRulesDto, SendEventRequestDto,
};
use tornado_engine_matcher::model::{
    ProcessedEvent, ProcessedFilter, ProcessedFilterStatus, ProcessedIterator, ProcessedNode,
    ProcessedRule, ProcessedRuleStatus, ProcessedRules,
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
            nodes: events
                .into_iter()
                .take(1)
                .flat_map(|n| n.result)
                .map(processed_node_into_dto)
                .collect::<Result<Vec<_>, _>>()?,
        },
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
