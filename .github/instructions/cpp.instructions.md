---
description: "Use when changing the C++20 WVX runtime, native rendering, settings, IPC, or CMake targets."
name: "WVX C++ Runtime"
applyTo: "apps/wvx-runtime/**/*.{h,hpp,c,cc,cpp,cxx}"
---

# Runtime C++ WVX

- Cibler C++20, CMake et le Windows SDK retenu par le dépôt ; garder les
  responsabilités séparées entre configuration, rendu, widgets et IPC.
- Utiliser RAII et des types de handles propriétaires ; limiter le code brut
  Win32 aux adaptateurs nécessaires et vérifier chaque résultat d'API.
- Garder les échanges Studio/runtime versionnés, bornés et validés. Ne jamais
  faire confiance aux données reçues depuis un pipe.
- Sauvegarder la configuration JSON sous les données utilisateur, avec
  validation et remplacement atomique ; préserver les réglages récupérables en
  cas d'erreur de lecture.
- Partager le device et les ressources GPU ; ne pas animer ou redessiner une
  surface statique sans changement visuel.
- Limiter les dépendances et signaler les API Windows indisponibles au lieu de
  prétendre qu'un effet fonctionne.
- Valider avec configuration, build et CTest CMake sous Windows/MSVC. Un build
  croisé n'est pas un test d'exécution Windows.
