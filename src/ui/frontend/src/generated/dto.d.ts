/* tslint:disable */
export type RuleDto = { name: string; description: string; continue: boolean; active: boolean; constraint: ConstraintDto; actions: ActionDto[] };

export type ConstraintDto = { WHERE: OperatorDto | null; WITH: { [ key: string ]: ExtractorDto } };

export type ExtractorDto = { from: string; regex: ExtractorRegexDto };

export type ExtractorRegexDto = { match: string; group_match_idx: number };

export type OperatorDto = 
 | { type: "AND"; operators: OperatorDto[] } 
 | { type: "OR"; operators: OperatorDto[] } 
 | { type: "contain"; text: string; substring: string } 
 | { type: "equal"; first: string; second: string } 
 | { type: "regex"; regex: string; target: string };

export type ActionDto = { id: string; payload: Value };

export type FilterDto = { name: string; description: string; active: boolean; filter: OperatorDto | null };

export type MatcherConfigDto = 
 | { Filter: { filter: FilterDto; nodes: { [ key: string ]: MatcherConfigDto } } } 
 | { Rules: { rules: RuleDto[] } };


export type Value = any;


export type EventDto = { type: string; created_ms: number; payload: { [ key: string ]: Value } };

