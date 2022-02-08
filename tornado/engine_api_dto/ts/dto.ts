


/* tslint:disable */

/* WARNING: this file was automatically generated at compile time */
/* DO NOT CHANGE IT MANUALLY */

/* ------------ */
/* custom types */
/* ------------ */

export type Value = any;


/* ---------------- */
/* 'common' types   */
/* ---------------- */

export type Id<T> = { id: T };

export type WebError = {     code: string; params: { [key: string]: string }; message: string |     null };


/* -------------- */
/* 'auth' types   */
/* -------------- */

export type Auth = { user: string; roles: string []; preferences: UserPreferences | null };

export type AuthWithPermissionsDto = {     user: string; permissions: PermissionDto []; preferences:     UserPreferences | null };

export enum PermissionDto {     ConfigEdit = "ConfigEdit", ConfigView = "ConfigView", RuntimeConfigEdit =     "RuntimeConfigEdit", RuntimeConfigView = "RuntimeConfigView",     TestEventExecuteActions = "TestEventExecuteActions" };

export type UserPreferences = { language: string | null };


/* -------------- */
/* 'config' types */
/* -------------- */

export type ActionDto = { id: string; payload: Value };

export type ConstraintDto = { WHERE: OperatorDto | null; WITH: { [key: string]: ExtractorDto } };

export type ExtractorDto = {     from: string; regex: ExtractorRegexDto; modifiers_post: ModifierDto     [] };

export type ExtractorRegexDto = 
 | {     type: "Regex"; match: string; group_match_idx: number | null;     all_matches: boolean | null } 
 | {     type: "RegexNamedGroups"; named_match: string; all_matches: boolean |     null } 
 | { type: "KeyRegex"; single_key_match: string };

export type FilterDto = { description: string; active: boolean; filter: OperatorDto | null };

export type MatcherConfigDraftDataDto = {     user: string; created_ts_ms: number; updated_ts_ms: number; draft_id: string };

export type MatcherConfigDraftDto = { data: MatcherConfigDraftDataDto; config: MatcherConfigDto };

export type MatcherConfigDto = 
 | {     type: "Filter"; name: string; filter: FilterDto; nodes:     MatcherConfigDto [] } 
 | { type: "Ruleset"; name: string; rules: RuleDto [] };

export type ModifierDto = 
 | { type: "Lowercase" } 
 | {     type: "Map"; mapping: { [key: string]: string }; default_value:     string | null } 
 | {     type: "ReplaceAll"; find: string; replace: string; is_regex:     boolean } 
 | { type: "ToNumber" } 
 | { type: "Trim" };

export type OperatorDto = 
 | { type: "AND"; operators: OperatorDto [] } 
 | { type: "OR"; operators: OperatorDto [] } 
 | { type: "NOT"; operator: OperatorDto } 
 | { type: "contains"; first: Value; second: Value } 
 | { type: "containsIgnoreCase"; first: Value; second: Value } 
 | { type: "equals"; first: Value; second: Value } 
 | { type: "equalsIgnoreCase"; first: Value; second: Value } 
 | { type: "ge"; first: Value; second: Value } 
 | { type: "gt"; first: Value; second: Value } 
 | { type: "le"; first: Value; second: Value } 
 | { type: "lt"; first: Value; second: Value } 
 | { type: "ne"; first: Value; second: Value } 
 | { type: "regex"; regex: string; target: string };

export type RuleDto = {     name: string; description: string; continue: boolean; active:     boolean; constraint: ConstraintDto; actions: ActionDto [] };

export type ProcessingTreeNodeConfigDto = 
 | {     type: "Filter"; name: string; rules_count: number; children_count:     number; description: string; active: boolean } 
 | { type: "Ruleset"; name: string; rules_count: number };

export type ProcessingTreeNodeDetailsDto = 
 | {     type: "Filter"; name: string; description: string; active: boolean; filter: OperatorDto | null } 
 | { type: "Ruleset"; name: string; rules: RuleDetailsDto [] };

export type RuleDetailsDto = {     name: string; description: string; continue: boolean; active:     boolean; actions: string [] };

export type TreeInfoDto = { rules_count: number; filters_count: number };


/* ------------- */
/* 'event' types */
/* ------------- */

export type EventDto = {     type: string; created_ms: number; metadata:     { [key: string]: Value }; payload: { [key: string]: Value } };

export enum ProcessType { Full = "Full", SkipActions = "SkipActions" };

export type ProcessedEventDto = { event: EventDto; result: ProcessedNodeDto };

export type ProcessedFilterDto = { status: ProcessedFilterStatusDto };

export enum ProcessedFilterStatusDto { Matched = "Matched", NotMatched = "NotMatched", Inactive = "Inactive" };

export type ProcessedNodeDto = 
 | {     type: "Filter"; name: string; filter: ProcessedFilterDto; nodes:     ProcessedNodeDto [] } 
 | { type: "Ruleset"; name: string; rules: ProcessedRulesDto };

export type ProcessedRuleDto = {     name: string; status: ProcessedRuleStatusDto; actions: ActionDto [];     message: string | null; meta: ProcessedRuleMetaData | null };

export type ProcessedRulesDto = { rules: ProcessedRuleDto []; extracted_vars: Value };

export enum ProcessedRuleStatusDto {     Matched = "Matched", PartiallyMatched = "PartiallyMatched", NotMatched =     "NotMatched", NotProcessed = "NotProcessed" };

export type SendEventRequestDto = { process_type: ProcessType; event: EventDto };


/* ---------------- */
/* 'matcher' types   */
/* ---------------- */

export type ActionMetaData = { id: string; payload: { [key: string]: EnrichedValue } };

export type EnrichedValue = { content: EnrichedValueContent; meta: ValueMetaData };

export type EnrichedValueContent = 
 | { type: "Single"; content: Value } 
 | { type: "Map"; content: { [key: string]: EnrichedValue } } 
 | { type: "Array"; content: EnrichedValue [] };

export type ProcessedRuleMetaData = { actions: ActionMetaData [] };

export type ValueMetaData = { modified: boolean; is_leaf: boolean };


/* -------------- */
/* 'runtime_config' types */
/* -------------- */

export type LoggerConfigDto = { level: string; stdout_enabled: boolean; apm_enabled: boolean };

export type SetLoggerApmRequestDto = { enabled: boolean };

export type SetLoggerLevelRequestDto = { level: string };

export type SetLoggerStdoutRequestDto = { enabled: boolean };

export type SetApmPriorityConfigurationRequestDto = { logger_level: string | null };

export type SetStdoutPriorityConfigurationRequestDto = {};