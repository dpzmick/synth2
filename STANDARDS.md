Unfortunately, `rustfmt` sort of sucks, so here's some rules to try to follow.
If anyone else ever actually commits code to this project, these rules will be
sort of enforced. As long as your code looks pretty reasonable and is consistent
with the glaringly obvious rules it will be okay.

### General:
- 4 spaces to indent (2 spaces are *correct* but lets try to at least look like
  other people's rust code)
- 80 character lines
- Use trailing commas for arrays and maps and other **data** declarations
  - trailing commas look funky to me in function calls though

```rust
let a = [
    1,
    2,
    3,
    4,
];
```

- Use vertical whitespace to create sections where appropriate, but never use
  more than one empty line.
- Some horizontal (alignment) whitespace is okay. Try to avoid it in places
  where it is likely to create lots of diff noise, but it does look nice so I
  won't complain too much about it.

### Imports:
- local imports come before system imports
- imports in alphabetical order
- try to prefix external packages with their "namespace" when reasonable
  - function local imports are fine, but try to keep it clear in top level
    declarations

### Function Declarations:
- If a return causes a line to be too long, drop the return to the next line and
  indent

```rust
fn register_input_port(&mut self, name: &PortName)
    -> Result<InputPortHandle<'a>, PortManagerError>;
```

- If the function argument list causes a function declaration to be too long,
  drop all of the args to their own lines. Line the return up with the closing
  paren:

```rust
fn long_function(
    i32 a,
    i32 b,
    i32 c,
    i32 d,
    i32 e,
    i32 f,
    i32 g
) -> i32;
```

### Function Calls:
- If a function call is too long, try to prefer dropping the arguments to the
  next line, instead of lining them up visually. It looks nice to line them up
  visually, but that just creates diff noise in the future unfortunately.

Good:

```rust
let x = call_bad_boy(
    a, b, c, d);

// if we can't fit all the args on the *next* line, one arg per line
let y = call_worse_boy(
    longArgName1,
    longArgName2,
    ...
    longArgNameN)
```

- Use your best judgement. Rust has lots of syntax!
