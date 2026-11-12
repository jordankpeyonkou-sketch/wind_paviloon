---
description: "Use when editing HLSL shaders or GPU effects in the WVX runtime."
name: "WVX HLSL"
applyTo: "apps/wvx-runtime/shaders/**/*.hlsl"
---

# Shaders HLSL

- Garder les shaders spécialisés, déterministes et documenter les conventions
  d'espace, les entrées, sorties, ressources et constantes.
- Borner les dimensions et paramètres avant tout accès ou calcul coûteux.
- Éviter les passes et transferts GPU redondants ; libérer les ressources avec
  le cycle de vie du pipeline.
- Valider les shaders avec le compilateur ciblé par la version du Windows SDK
  et ajouter des tests de rendu ou une validation de compilation quand
  l'environnement le permet.
- Distinguer une surface matériau WVX d'une capture réelle du bureau ; ne pas
  revendiquer de flou d'arrière-plan si aucune source d'image n'est fournie.
