# Executor Common

The *tornado_executor_common* crate contains the Trait definitions for the Executors.

An Executor is in charge of performing a specific Action (usually only one, but sometimes more).
It receives an action description from the Tornado engine and delivers the operation linked to it.
