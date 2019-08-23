 
# Tornado Backend DTOs

The __tornado_backend_dto__ component contains the 
[Data Transfer Object](https://en.wikipedia.org/wiki/Data_transfer_object) definitions
to carry data between processes. 

These DTOs are the structures exposed by the REST endpoints of the Tornado backend.

The object structures are defined in the Rust programming language and built as a Rust crate.
In addition, at build time, in the _ts_ subfolder, 
[Typescript](https://www.typescriptlang.org/) definitions 
of the defined DTOs are generated.

These Typescript definitions can be imported by API clients written in Typescript
to provide compile-time type safety.

## Generate the DTO Typescript definition files:

To generate the Typescript definitions files corresponding to the Rust structures,
execute the tests of this crate with the environment variable
**TORNADO_DTO_BUILD_REGENERATE_TS_FILES** set to *true*.

For example:
```bash
TORNADO_DTO_BUILD_REGENERATE_TS_FILES=true cargo test 
```

The resulting _ts_ will be generated in the **ts** subfolder.