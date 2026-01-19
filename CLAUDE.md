# CLAUDE.md

## Project Overview

Sorcery Desktop is an open source, MIT license, cross-platform, hyperlinker for editors/IDE. The srcuri:// protocol (also known as the "Sorcery protocol") lets developers share code references in an editor-independent manner that links to what developers like--the file and line, but *in their own editor*. Without Sorcery links, developers share github links, but opening github.com doesn't help you debug and edit code.

For example:
* srcuri://myproject/README.md:75

With Sorcery Desktop installed, and the srcuri:// protocol registered, devs can open their own editor to the exact file and line with a click.

---
# Overview:

* This project keeps a running server process. It uses this to track active editors, and when they were last seen.

## EditorManagers

Each editor gets a custom EditorManager. The EditorManager

--- 
Instructions:

* Act as a principal architect level developer.
* Don't make comments that repeat the meaning of what the code says, but in the same or slightly different words.
* No fallbacks! If you are using a fallback, really question if you can just remove the thing that is failing to work.
* Check for existing implementations to reuse *before* you add new methods.


## Code Discipline
- Avoid one-off fallbacks; discuss before adding behavior switches.
- Remove dead/legacy code rather than commenting it out.
- When you remove code, don't add a comment about it.
- Write intent-revealing code, not comments that restate logic.
- I want #2, but for development, and debugging, #1 is also useful.

## Documentation Maintenance

When adding a new feature, as a final step:
1. Update `dev/features.md` - add the feature under the appropriate section
2. Update `dev/use-cases.md` - if the feature represents a new use case (use "As a..." format)