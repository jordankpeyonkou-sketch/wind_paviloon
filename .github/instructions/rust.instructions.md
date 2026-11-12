---
description: "Use only when maintaining the historical GlassDesk Rust prototype during the WVX migration."
name: "Legacy GlassDesk Rust"
applyTo: "crates/**/*.rs"
---

# Maintenance du prototype Rust historique

Le code `crates/` est le prototype GlassDesk pré-migration. Les nouvelles
fonctionnalités WVX doivent être implémentées dans la stack C++/CMake ou
C#/WinUI 3 définie par les consignes racine, pas dans ce prototype.

- Suivre l'édition et les conventions de `Cargo.toml`; garder le typage
  explicite et éviter `unwrap`, casts non justifiés, état global et allocations
  répétées sur les chemins fréquents.
- Garder les fonctions et modules ciblés. Préserver les frontières `core`,
  `widget-host` et `render` et les conditions de compilation par plateforme.
- Propager les erreurs avec leur contexte. Ne pas convertir silencieusement
  une erreur d'E/S, GPU, configuration ou fenêtre en succès.
- Pour les données persistantes, valider les valeurs, prévoir une version de
  schéma et écrire de manière atomique. Ajouter des tests de lecture, de
  validation et de mise à jour lorsque ces formats changent.
- Limiter les opérations non bornées et les travaux périodiques. Suspendre les
  mises à jour visuelles lorsque les données n'ont pas changé.
- Localiser le `unsafe`, documenter ses invariants et vérifier les résultats
  des API natives.
- Ajouter des tests de comportement aux changements importants. Tester aussi
  les erreurs, les limites et les retours arrière pris en charge.
- Après les modifications, lancer `cargo fmt --all -- --check`,
  `cargo check --workspace` et `cargo test --workspace`. Utiliser un
  toolchain Windows GNU explicitement seulement lorsque l'environnement ne
  possède pas le linker MSVC.
