# Implementation details

## Matcher crate

The Matcher contains the core logic of the Tornado Engine. It is in charge of:
- Receiving events from the collectors
- Process incoming events and detect which rule they satisfy
- Trigger the expected actions

Due to its strategical position, its performance is of utmost importance for the global throughput.

The internal code structure is kept simple on purpose, and the final objective is reached by splitting the global process in a set of modular, isolated and well-tested blocks of logic. Each "block" communicates with the others through a well-defined API hiding, at the same time, its internal implementation.

This modularization effort is twofold; first, it minimises the risk that local changes have a global impact; and, second, it disjoins functional and technical complexity; so, increasing functional complexity does not result in increasing code complexity. As a consequence, the maintenance and evolutionary costs are expected to be linear in the short, mid and long term.

From a very high level point of view, when the matcher starts up, it follows these phases:

- Configuration (code in the "config" module): The configuration phase loads a set of files from the file system. Each file is a rule represented in JSON format. The outcome of this phase is an array of Rule instances created from the JSON files.
- Validation (code in the "validator" module): it receives an array of Rules and verifies that they all respect some predefined constraints (e.g. the rule name cannot contain dots). The output is the same array as input.
- Builds the Matcher (code in the "matcher" module): receives a list of rules and for each rule:
    - builds the Accessors for accessing the event properties (using the AccessorBuilder, code in the "accessor" module)
    - builds the Operator for evaluating whether an event matches the "WHERE" clause of the rule (using the OperatorBuilder, code in the "operator" module)
    - builds the Extractors for generating the user-defined variables (using the ExtractorBuilder, code in the "extractor" module)

        This phase's output is an instance of the Matcher that contains all the required logic to process an event against all the defined rules.
        A matcher is stateless and thread-safe; so, a single instance can be used to serve the entire application load.
- Start listening for incoming events: TO BE IMPLEMENTED.