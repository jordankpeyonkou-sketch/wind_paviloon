---
description: "Use when changing Win32 APIs, WVX runtime windows, DWM, shell integration, DPI, or Windows-specific runtime behavior."
name: "WVX Windows Native"
applyTo: "apps/wvx-runtime/src/**"
---

# Intégrations Windows natives WVX

- Utiliser les API publiques et documentées lorsqu'elles répondent au besoin.
- Conserver toute intégration de shell expérimentale facultative et réversible ;
  fournir un mode autonome qui n'en dépend pas.
- Ne pas injecter de DLL, installer de hooks dans des processus tiers, patcher
  DWM, remplacer des composants du shell ou modifier le registre.
- Contrôler les résultats Win32 et préserver l'erreur système. Rester prudent
  avec les handles, styles, coordonnées, DPI et écrans multiples.
- Vérifier le comportement sur Windows et ne pas présenter un test de
  compilation croisée comme un test d'exécution ou de compatibilité complète.
