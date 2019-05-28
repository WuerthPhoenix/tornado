use crate::api::handler::SendEventRequest;
use dto::config::ActionDto;
use dto::event::{
    EventDto, ProcessType, ProcessedEventDto, ProcessedFilterDto, ProcessedFilterStatusDto,
    ProcessedNodeDto, ProcessedRuleDto, ProcessedRuleStatusDto, ProcessedRulesDto,
    SendEventRequestDto,
};
use serde_json::Error;
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
use tornado_common_api::Action;
use tornado_engine_matcher::model::{
    InternalEvent, ProcessedEvent, ProcessedFilter, ProcessedFilterStatus, ProcessedNode,
    ProcessedRule, ProcessedRuleStatus, ProcessedRules,
};

pub fn dto_into_send_event_request(dto: SendEventRequestDto) -> Result<SendEventRequest, Error> {
    Ok(SendEventRequest {
        process_type: match dto.process_type {
            ProcessType::Full => crate::api::handler::ProcessType::Full,
            ProcessType::SkipActions => crate::api::handler::ProcessType::SkipActions,
        },
        event: serde_json::from_value(serde_json::to_value(dto.event)?)?,
    })
}

pub fn processed_event_into_dto(
    processed_event: ProcessedEvent,
) -> Result<ProcessedEventDto, Error> {
    Ok(ProcessedEventDto {
        event: internal_event_into_dto(processed_event.event)?,
        result: processed_node_into_dto(processed_event.result)?,
    })
}

pub fn internal_event_into_dto(internal_event: InternalEvent) -> Result<EventDto, Error> {
    let event_value: tornado_common_api::Value = internal_event.into();
    let dto = serde_json::from_value(serde_json::to_value(event_value)?)?;
    Ok(dto)
}

pub fn processed_node_into_dto(node: ProcessedNode) -> Result<ProcessedNodeDto, Error> {
    Ok(match node {
        ProcessedNode::Rules { rules } => {
            ProcessedNodeDto::Rules { rules: processed_rules_into_dto(rules)? }
        }
        ProcessedNode::Filter { filter, nodes } => ProcessedNodeDto::Filter {
            nodes: nodes
                .into_iter()
                .map(|(key, value)| {
                    let dto = processed_node_into_dto(value)?;
                    Ok((key, dto))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?,
            filter: processed_filter_into_dto(filter),
        },
    })
}

pub fn processed_rules_into_dto(node: ProcessedRules) -> Result<ProcessedRulesDto, Error> {
    Ok(ProcessedRulesDto {
        extracted_vars: node
            .extracted_vars
            .into_iter()
            .map(|(key, value)| {
                let dto = serde_json::to_value(value)?;
                Ok((key, dto))
            })
            .collect::<Result<HashMap<_, _>, _>>()?,
        rules: node
            .rules
            .into_iter()
            .map(|(key, value)| {
                let dto = processed_rule_into_dto(value)?;
                Ok((key, dto))
            })
            .collect::<Result<HashMap<_, _>, _>>()?,
    })
}

pub fn processed_rule_into_dto(node: ProcessedRule) -> Result<ProcessedRuleDto, Error> {
    Ok(ProcessedRuleDto {
        message: node.message,
        rule_name: node.rule_name,
        actions: node.actions.into_iter().map(action_into_dto).collect::<Result<Vec<_>, _>>()?,
        status: processed_rule_status_into_dto(node.status),
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
    ProcessedFilterDto { name: node.name, status: processed_filter_status_into_dto(node.status) }
}

pub fn processed_filter_status_into_dto(node: ProcessedFilterStatus) -> ProcessedFilterStatusDto {
    match node {
        ProcessedFilterStatus::NotMatched => ProcessedFilterStatusDto::NotMatched,
        ProcessedFilterStatus::Matched => ProcessedFilterStatusDto::Matched,
        ProcessedFilterStatus::Inactive => ProcessedFilterStatusDto::Inactive,
    }
}
