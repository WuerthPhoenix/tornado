.. _tornado-matcher-details:

Matcher Engine Implementation Details
`````````````````````````````````````

Matcher Crate
+++++++++++++

The Matcher crate contains the core logic of the Tornado Engine. It is
in charge of: - Receiving events from the collectors - Processing
incoming events and detecting which filter and rule they satisfy -
Triggering the expected actions

Due to its strategic position, its performance is of utmost importance
for global throughput.

The code’s internal structure is kept simple on purpose, and the final
objective is reached by splitting the global process into a set of
modular, isolated and well-tested blocks of logic. Each “block”
communicates with the others through a well-defined API, which at the
same time hides its internal implementation.

This modularization effort is twofold; first, it minimizes the risk that
local changes will have a global impact; and second, it separates
functional from technical complexity, so that increasing functional
complexity does not result in increasing code complexity. As a
consequence, the maintenance and evolutionary costs of the code base are
expected to be linear in the short, mid- and long term.

At a very high level view, when the matcher initializes, it follows
these steps:

-  Configuration (see the code in the “config” module): The
   configuration phase loads a set of files from the file system. Each
   file is a filter or a rule in JSON format. The outcome of this step
   is a processing tree composed of filter and rule configurations
   created from the JSON files.
-  Validation (see the code in the “validator” module): The Validator
   receives the processing tree configuration and verifies that all
   nodes respect a set of predefined constraints (e.g., the identifiers
   cannot contain dots). The output is either the same processing tree
   as the input, or else an error.
-  Match Preparation (see the code in the “matcher” module): The Matcher
   receives the processing tree configuration, and for each node:

   -  if the node is a filter:

      -  Builds the Accessors for accessing the event properties using
         the AccessorBuilder (see the code in the “accessor” module).
      -  Builds an Operator for evaluating whether an event matches the
         filter itself (using the OperatorBuilder, code in the
         “operator” module).

   -  if the node is a rule:

      -  Builds the Accessors for accessing the event properties using
         the AccessorBuilder (see the code in the “accessor” module).
      -  Builds the Operator for evaluating whether an event matches the
         “WHERE” clause of the rule (using the OperatorBuilder, code in
         the “operator” module).
      -  Builds the Extractors for generating the user-defined variables
         using the ExtractorBuilder (see the code in the “extractor”
         module). This step’s output is an instance of the Matcher that
         contains all the required logic to process an event against all
         the defined rules. A matcher is stateless and thread-safe, thus
         a single instance can be used to serve the entire application
         load.

-  Listening: Listen for incoming events and then match them against the
   stored filters and rules.
