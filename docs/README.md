Diagram made with https://tabatkins.github.io/railroad-diagrams/generator.html and this source code:

```python
Diagram(
 OneOrMore(
Sequence(
  Group(Choice(1,
    Terminal("--startdoc"),
    Sequence(Terminal("-s/--start"), Terminal("TAG")),
    Sequence(Terminal("-e/--end"), Terminal("TAG")),
    Terminal("--enddoc"),
  ), "SAX event"),
  Group(OneOrMore(
   Choice(1,
    Sequence(Terminal("-o"), Terminal("RAW_TEXT")),
    Sequence(Terminal("-v"), Terminal("XML_ATTRIBUTE")),
    Sequence(Terminal("-V"), Terminal("XML_ATTRIBUTE"), Terminal("DEFAULT_IF_MISSING")),
    Sequence(Terminal("--tab")),
    Sequence(Terminal("--nl")),
   )
  ), "Output")
)))
```
