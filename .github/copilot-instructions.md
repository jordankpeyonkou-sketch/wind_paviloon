# WVX — Consignes du dépôt

## Langue et mission

- Répondre en français pour les explications, plans et comptes rendus ; nommer
  les symboles et fichiers de code en anglais.
- Le produit cible est **WVX (Windows Visual Experience)**, une application
  native de widgets et de personnalisation visuelle pour Windows 11.
- La migration depuis le prototype GlassDesk/Rust vers C++/CMake et C#/WinUI 3
  est en cours. Les dossiers `crates/` et le workspace Cargo sont historiques
  tant qu'ils ne sont pas supprimés après validation de leurs remplacements.
- `strucrure/copilot-instructions.md` est la référence produit et technique
  WVX. `strucrure/GlassDesk_Architecture.md` décrit l'ancien prototype Rust et
  ne définit plus la stack cible.

## Stack cible et architecture

- Runtime natif : C++20, CMake et MSVC ; Windows 11 x64.
- Studio : C# avec WinUI 3 / Windows App SDK, hébergé dans
  `apps/wvx-studio/`.
- Runtime et rendu : `apps/wvx-runtime/`; préférer Direct2D pour les éléments
  2D et Direct3D 11 pour les effets GPU nécessaires.
- Configurations : JSON versionné, validé, sauvegardé atomiquement sous les
  données utilisateur.
- Contrat Studio/runtime : Named Pipe local documenté, messages JSON
  versionnés, entrées et tailles vérifiées.
- Respecter les responsabilités des projets. Le Studio ne dessine pas les
  widgets ; il pilote le runtime via IPC.

## Règles d'implémentation

- Construire par incréments vérifiables ; le MVP prioritaire est un runtime
  natif, une horloge personnalisable, une surface de matériau, la configuration
  persistante et un contrôle Studio minimal. Ne pas scaffolder les phases
  futures prématurément.
- Préserver les changements existants et le comportement utile. Ne retirer le
  prototype Rust qu'après validation du remplacement pertinent.
- Utiliser les API Windows documentées, gérer les erreurs et capacités
  indisponibles, et garder les intégrations expérimentales facultatives.
- Ne pas injecter de DLL, installer de hooks dans des processus tiers, patcher
  DWM ou modifier le shell, le registre ou les fichiers système.
- Valider tous les paramètres et messages IPC ; borner les tailles et éviter
  les écritures de fichiers non atomiques.
- Favoriser le rendu événementiel, le partage des ressources GPU et la
  libération déterministe des ressources.
- Ne jamais annoncer une capacité qui n'est pas implémentée et testée. Les
  widgets, effets de bureau et intégrations non disponibles doivent être
  présentés explicitement comme tels.
- Toute nouvelle dépendance doit être justifiée par sa fonction, sa licence,
  son impact sur la distribution et l'absence d'une API système adaptée.

## Validation

Après les changements natifs :

```powershell
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```

Valider le Studio séparément avec `dotnet build` sur son fichier projet. Ne pas
confondre une configuration CMake, un build croisé ou une compilation avec un
test d'exécution Windows. Si Visual Studio/MSVC, le Windows SDK, CMake ou
Windows App SDK manque, le signaler et donner le résultat réellement obtenu.

Inspecter le diff final. Résumer les fichiers migrés, les commandes réellement
exécutées et les capacités ou vérifications restant à faire.
