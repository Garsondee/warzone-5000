# Body dominance analysis

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generator: `python3 -I tools/research/wz_components.py <wz_checkout> dominance`.

### Strict Pareto dominance among designable bodies

A body is *dominated* if another body is at least as good on HP, both armours and engine power AND at least as light and cheap (power + build points), and strictly better somewhere.

No body is strictly dominated once cost and weight are counted: every body trades something for something.

### Dominance ignoring cost and weight (raw capability only)

- Vengeance: beaten on every raw stat by 2 bodies (Wyvern, Dragon)
- Python: beaten on every raw stat by 3 bodies (Vengeance, Wyvern, Dragon)
- Mantis: beaten on every raw stat by 3 bodies (Vengeance, Wyvern, Dragon)
- Wyvern: beaten on every raw stat by 1 bodies (Dragon)
- Dragon: beaten on every raw stat by 0 bodies
- Viper: beaten on every raw stat by 12 bodies (Vengeance, Python, Mantis, Wyvern, Dragon, Leopard, Retaliation, Cobra, Panther, Retribution, Scorpion, Tiger)
- Leopard: beaten on every raw stat by 9 bodies (Vengeance, Python, Mantis, Wyvern, Dragon, Cobra, Panther, Retribution, Tiger)
- Retaliation: beaten on every raw stat by 5 bodies (Vengeance, Wyvern, Dragon, Retribution, Tiger)
- Bug: beaten on every raw stat by 10 bodies (Vengeance, Python, Mantis, Wyvern, Dragon, Retaliation, Panther, Retribution, Scorpion, Tiger)
- Cobra: beaten on every raw stat by 8 bodies (Vengeance, Python, Mantis, Wyvern, Dragon, Panther, Retribution, Tiger)
- Panther: beaten on every raw stat by 5 bodies (Vengeance, Python, Wyvern, Dragon, Tiger)
- Retribution: beaten on every raw stat by 3 bodies (Vengeance, Wyvern, Dragon)
- Scorpion: beaten on every raw stat by 6 bodies (Vengeance, Mantis, Wyvern, Dragon, Retribution, Tiger)
- Tiger: beaten on every raw stat by 3 bodies (Vengeance, Wyvern, Dragon)
