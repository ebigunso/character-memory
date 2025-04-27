# Project Guidelines

## Coding Philosophy
- When coding, always ask yourself: are you sure this is the best way to implement this?
- Before invoking any tool, write down your intended changes so your future self knows what you aimed to achieve.
- If a replace_in_file fails, try again with smaller incremental changes before falling back to write_in_file.

## Module Organization
- Use a file named after the module to declare submodules, instead of using mod.rs files.
  Example: For a directory named 'models', use 'models.rs' to declare its submodules instead of 'models/mod.rs'
- Place each entity (struct, enum, etc.) in its own module file

## Code Style & Patterns
- Only mark functions as fully public if they belong to the consumer-facing API; otherwise, use private or `pub(crate)`. Always consider the function’s intended usage before setting its visibility.
