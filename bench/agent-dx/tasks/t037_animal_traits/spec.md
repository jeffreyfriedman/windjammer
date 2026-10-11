Define an `Animal` abstraction with `name`, `sound` and `legs` and a DEFAULT method `describe` that returns `<name> the <kind> says <sound> on <legs> legs`. One kind (`cat`) overrides `describe` to return `<name> the cat purrs softly`. Kinds and their data:

- `dog`: sound `woof`, 4 legs
- `cat`: sound `meow`, 4 legs (custom describe above)
- `duck`: sound `quack`, 2 legs
- `snake`: sound `hiss`, 0 legs; the default describe must say `on 0 legs`
- `spider`: sound `...`, 8 legs

Each input line is `kind name`. Print `describe()` for each line. After the last line print `legs total=<sum of legs>`. Lines with an unknown kind print `unknown kind <kind>` and contribute 0 legs.
