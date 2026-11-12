---
description: "Use when editing the WVX Studio C# / WinUI 3 application."
name: "WVX Studio WinUI"
applyTo: "apps/wvx-studio/**/*.{cs,xaml}"
---

# WVX Studio

- Garder la logique métier et Named Pipe dans des services C# séparés des vues
  XAML.
- Maintenir une interface réactive ; les appels IPC et accès fichiers doivent
  être asynchrones et bornés.
- Représenter explicitement les états runtime indisponible, chargement, erreur
  et sauvegarde ; ne pas simuler un succès lorsque le pipe est absent.
- Partager avec le runtime le contrat JSON versionné. Valider les valeurs avant
  envoi et afficher les erreurs de validation retournées.
- Marquer comme indisponibles les widgets et effets qui ne sont pas encore
  pris en charge par le runtime.
- Vérifier séparément le build du projet WinUI 3 avec le Windows App SDK
  correspondant.
