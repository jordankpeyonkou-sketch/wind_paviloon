# WVX — Instructions de développement pour GitHub Copilot

> **Rôle de ce fichier :** fournir à GitHub Copilot le contexte produit, les décisions d'architecture et les règles d'implémentation du projet WVX. Ces consignes s'appliquent à l'ensemble du dépôt.
>
> **Langue :** répondre en français dans les explications, plans et comptes rendus. Utiliser l'anglais pour les noms de classes, fonctions, variables, fichiers, interfaces et API, sauf convention déjà établie dans le dépôt.

---

## 1. Mission du projet

**WVX (Windows Visual Experience)** est une plateforme native de personnalisation visuelle pour Windows 11. Elle doit permettre de créer et d'exécuter des widgets de bureau personnalisés, de composer des matériaux graphiques (Frosted Glass, Liquid Glass, Mica, Acrylic), d'animer les surfaces WVX et, dans la mesure permise par les API publiques de Windows, d'étendre certains effets à l'environnement du bureau.

WVX n'est pas un remplacement du shell Windows et ne doit pas dépendre de modifications non prises en charge du système.

### Objectifs produit

1. Afficher des widgets personnalisables sur le bureau Windows.
2. Fournir une horloge avancée capable d'adapter sa composition au fond d'écran.
3. Fournir un moteur de matériaux et d'effets réutilisable par tous les widgets et surfaces WVX.
4. Fournir un moteur d'animations indépendant des widgets.
5. Fournir WVX Studio, l'application de configuration et de création.
6. Permettre des intégrations Windows compatibles et réversibles, sans compromettre la stabilité du système.
7. Maintenir une consommation CPU, RAM et GPU aussi faible que possible au repos.

### Priorité produit actuelle

**Ne pas chercher à implémenter toute la vision en une seule fois.** Le premier objectif est un MVP fiable comprenant :

- un runtime natif Windows ;
- une fenêtre/surface de widget transparente sur le bureau ;
- un gestionnaire de widgets minimal ;
- un widget horloge personnalisable ;
- un premier matériau Frosted Glass accéléré par GPU ;
- une configuration persistante ;
- une gestion correcte du cycle de vie et de la consommation des ressources.

Les effets Liquid Glass avancés, la détection de sujets, les intégrations globales à la barre des tâches et les personnalisations de l'écran de verrouillage ne sont pas des prérequis du MVP.

---

## 2. Règles de travail pour Copilot

Avant toute modification :

1. **Inspecter le dépôt réel.** Lire le `README`, les fichiers de build, l'arborescence, les conventions existantes et les tests avant de proposer ou créer des fichiers.
2. Vérifier les technologies, versions de SDK, dépendances et commandes réellement présentes. Ne pas inventer une structure existante.
3. Réutiliser les abstractions déjà présentes lorsqu'elles sont saines. Ne pas réécrire un module fonctionnel sans nécessité démontrée.
4. Définir le périmètre de la tâche et identifier les modules impactés avant de modifier le code.
5. Effectuer des changements petits, cohérents et compilables. Éviter les réécritures massives et le scaffolding prématuré.
6. Ne pas créer de fichiers vides, de classes factices ou de TODO sans utilité concrète.
7. Ne pas déclarer une fonctionnalité terminée si elle n'est pas implémentée, compilée et testée.
8. Après modification, lancer les vérifications adaptées : formatage, compilation, tests unitaires et tests d'intégration disponibles.
9. Rapporter clairement les fichiers modifiés, les commandes exécutées, les résultats et les limites restantes.
10. En cas d'incertitude sur une API Windows, consulter la documentation Microsoft correspondant au SDK ciblé avant de coder.

### Comportement attendu

- Si une demande est trop large, la découper en étapes techniques ordonnées et commencer par la plus petite étape utile.
- Si une information manque mais qu'une hypothèse raisonnable est possible, documenter cette hypothèse et avancer sans bloquer inutilement.
- Ne jamais prétendre avoir compilé ou testé un code sans avoir exécuté la commande correspondante.
- Ne pas modifier les paramètres globaux de Windows, le registre ou les fichiers système sans nécessité explicite, justification, sauvegarde et mécanisme de restauration.
- Préserver les modifications préexistantes de l'utilisateur. Ne jamais réinitialiser ou écraser arbitrairement des fichiers.

---

## 3. Plateforme et décisions technologiques

### Plateforme cible

- Système cible initial : **Windows 11 x64**.
- Définir et documenter la version minimale de Windows prise en charge dès que le projet fixe son Windows SDK et son Windows App SDK.
- Vérifier les capacités à l'exécution lorsque les API ou effets varient selon la version de Windows ou le matériel.
- Prévoir une dégradation propre lorsque le GPU, un effet ou une API n'est pas disponible.

### Stack de référence

| Domaine | Technologie de référence | Responsabilité |
|---|---|---|
| Runtime natif | C++20 ou C++23, selon le compilateur/SDK retenu | Cycle de vie, fenêtres, services, IPC et performances |
| Build natif | CMake + MSVC | Compilation et gestion des cibles natives |
| Rendu 2D | Direct2D | Dessin 2D et surfaces compatibles |
| Rendu GPU | Direct3D 11 au départ | Textures, shaders et effets personnalisés |
| Composition native | DirectComposition et/ou Windows Composition | Composition des surfaces et animations selon le cas d'usage |
| Application Studio | C# + WinUI 3 | Interface de configuration et d'édition |
| Widgets web | HTML + CSS + TypeScript | Création rapide de widgets personnalisés |
| Runtime web | WebView2, de manière mutualisée et contrôlée | Exécution des widgets web |
| Shaders | HLSL | Flou, distorsion, réfraction, dispersion et matériaux |
| Configuration | JSON avec schémas versionnés | Paramètres, manifestes, thèmes et migrations |
| Tests natifs | Framework C++ retenu par le dépôt, ou tests CTest | Tests unitaires et intégration |
| Tests Studio/web | Outils déjà adoptés par les sous-projets | Tests UI et widgets |

### Règles de choix technologique

- Ne pas utiliser Electron pour le runtime principal.
- Ne pas embarquer un moteur Chromium distinct par widget. Mutualiser les environnements WebView2 lorsque l'architecture et l'isolation le permettent ; contrôler le nombre de contrôles et de processus.
- Garder les widgets natifs possibles pour les usages très légers ou sensibles aux performances.
- Ne pas mélanger les API DirectComposition et Windows Composition comme si elles étaient interchangeables. Encapsuler leurs particularités derrière une abstraction de rendu claire.
- Ne pas ajouter une dépendance lourde pour une fonctionnalité qui peut être fournie simplement par les API Windows déjà utilisées.
- Justifier toute nouvelle dépendance par son utilité, son coût, sa licence et son impact sur la taille/distribution.
- Ne pas migrer de Direct3D 11 vers Direct3D 12 ou Vulkan sans besoin mesuré et décision d'architecture explicite.

---

## 4. Architecture logique cible

L'architecture est modulaire. Les modules communiquent par des interfaces définies et ne doivent pas accéder directement aux détails internes des autres modules.

```text
WVX
├── apps
│   ├── wvx-studio              # Application de configuration C# / WinUI 3
│   ├── wvx-runtime             # Processus natif de widgets et surfaces
│   └── wvx-screensaver         # Économiseur d'écran, phase ultérieure
│
├── core
│   ├── application             # Démarrage et arrêt du runtime
│   ├── configuration           # Lecture, validation, migration et sauvegarde JSON
│   ├── logging                 # Journalisation structurée
│   ├── event-system            # Événements internes
│   ├── ipc                     # Communication Studio / Runtime
│   ├── resource-manager        # Ressources CPU/GPU et cycle de vie
│   └── performance-monitor     # Mesure et adaptation des ressources
│
├── engines
│   ├── widget-engine           # Registre, runtime, rendu, layout et cycle de vie
│   ├── visual-effects-engine   # Matériaux, shaders, surfaces et composition
│   ├── animation-engine        # Timeline, keyframes, easing et transitions
│   ├── adaptive-clock-engine   # Horloge et adaptation au fond d'écran
│   └── windows-integration     # Intégrations Windows supportées
│
├── runtimes
│   ├── webview2                # Adaptateur du runtime de widgets web
│   └── native-widget           # Adaptateur pour widgets natifs
│
├── widgets                     # Widgets livrés avec WVX
├── shaders                     # Sources HLSL et variantes compilées
├── themes                      # Thèmes et presets de matériaux
├── shared                      # Contrats, modèles et tokens partagés
├── tests                       # Tests unitaires, intégration et rendu
├── docs                        # Architecture, API, décisions et roadmap
└── installer                   # Packaging et installation
```

Cette arborescence est une **cible logique**, pas une obligation de créer immédiatement tous ces dossiers. Créer uniquement les éléments nécessaires à la tâche en cours.

### Règles de dépendances

- `core` ne dépend pas de `apps` ni de l'interface WinUI.
- `engines` dépendent de contrats partagés et de services explicitement injectés ; éviter les dépendances circulaires.
- `widgets` consomment les API publiques de WVX, jamais les détails internes du runtime.
- `apps/wvx-studio` communique avec `wvx-runtime` via IPC documentée, pas via des accès directs à la mémoire du runtime.
- Les adaptateurs Windows sont isolés derrière des interfaces afin de pouvoir tester les moteurs sans dépendre de toutes les API du système.
- Les types partagés entre C++ et C# doivent être définis dans des contrats versionnés et sérialisables.

---

## 5. Widget Engine

Le Widget Engine gère l'enregistrement, le chargement, l'exécution, le rendu, le positionnement et le cycle de vie des widgets.

### Composants attendus

- `WidgetRegistry` : découvre et enregistre les widgets installés.
- `WidgetManifest` : représente et valide le manifeste d'un widget.
- `WidgetManager` : crée, déplace, redimensionne, active et supprime les instances.
- `WidgetRuntime` : interface commune aux runtimes web et natifs.
- `WidgetRenderer` : coordonne le rendu des widgets.
- `WidgetLayout` : gère position, dimensions, ancrage et écrans.
- `WidgetLifecycle` : gère les états `Created`, `Loading`, `Active`, `Suspended`, `Error` et `Disposed`.
- `WidgetPermissions` : applique les autorisations déclarées dans le manifeste.
- `WidgetEventBus` : distribue les événements autorisés.

### Exemple de manifeste de widget

Le schéma final doit être validé par JSON Schema et versionné. Exemple de départ :

```json
{
  "schemaVersion": 1,
  "id": "wvx.adaptive-clock",
  "name": "Adaptive Clock",
  "version": "0.1.0",
  "runtime": "web",
  "entry": "index.html",
  "window": {
    "width": 480,
    "height": 220,
    "resizable": true,
    "transparent": true
  },
  "permissions": [
    "system.time",
    "wallpaper.read"
  ],
  "rendering": {
    "targetFps": 30,
    "hardwareAcceleration": true
  }
}
```

Ne pas considérer cet exemple comme un contrat définitif tant que le schéma et les validations correspondantes ne sont pas présents dans le code.

### Règles de développement des widgets

- Un widget doit déclarer un identifiant stable, une version, un runtime, un point d'entrée et les permissions nécessaires.
- Les paramètres utilisateur doivent être séparés du manifeste et sauvegardés dans une configuration d'instance.
- Ne jamais donner à un widget un accès Win32 arbitraire.
- Exposer uniquement une API publique restreinte et versionnée.
- Valider toutes les données reçues par IPC ou par l'API JavaScript.
- Isoler les erreurs d'un widget : un widget défaillant ne doit pas arrêter le runtime ni les autres widgets.
- Gérer explicitement le DPI, le scaling, les écrans multiples, les changements de résolution et les coordonnées négatives des écrans secondaires.
- Gérer les interactions souris/clavier, le focus et les fenêtres non activables sans empêcher l'utilisateur d'interagir avec le bureau.

### Performance et cycle de vie

- Ne pas conserver une boucle de rendu active lorsqu'un widget n'a rien à afficher.
- Suspendre ou réduire le travail des widgets masqués, minimisés ou hors écran lorsque c'est sûr.
- Un widget horloge ne doit pas redessiner 60 fois par seconde si ses seules données changent chaque seconde.
- Les mises à jour de données et les mises à jour graphiques doivent être dissociées.
- Utiliser des caches pour les ressources réutilisables et libérer les ressources lorsqu'une instance est détruite.
- Prévoir une limite de fréquence par widget et un mécanisme global de budget de rendu.

---

## 6. Widgets web : HTML, CSS et TypeScript

HTML/CSS/TypeScript sont le langage de création des widgets web, pas le moteur graphique principal de WVX.

### Règles CSS

- Utiliser d'abord les propriétés CSS standard et les solutions compatibles avec le runtime ciblé.
- Les propriétés personnalisées WVX doivent être définies comme une API documentée, par exemple avec des variables CSS `--wvx-*`, des attributs `data-wvx-*` ou un fichier de thème.
- Ne jamais inventer une fonction CSS telle que `wvx-glass(...)` en prétendant qu'elle est nativement interprétée par le navigateur.
- Le runtime doit lire et valider les paramètres WVX, puis les transmettre au moteur natif via un contrat documenté.
- `backdrop-filter` peut être utilisé comme amélioration locale si disponible, mais ne constitue pas le moteur global de matériaux WVX.
- Fournir un fallback visuel si un effet CSS ou GPU n'est pas disponible.

### Exemple de déclaration de matériau

```css
.clock-surface {
  --wvx-material: frosted-glass;
  --wvx-blur-radius: 24px;
  --wvx-tint-opacity: 0.16;
  --wvx-refraction-strength: 0;

  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 28px;
  background: rgb(255 255 255 / 12%);
}
```

Le moteur natif peut interpréter les variables `--wvx-*` selon le contrat prévu. Le CSS doit rester présentable même si ces extensions ne sont pas interprétées.

### Sécurité WebView2

- Restreindre les navigations aux origines et ressources autorisées.
- Désactiver ou intercepter les nouvelles fenêtres non autorisées.
- Limiter l'accès aux fichiers locaux et aux protocoles personnalisés.
- Ne pas exposer une interface native générale à JavaScript.
- Exposer une API minimale, typée, validée et liée aux permissions du widget.
- Appliquer une Content Security Policy adaptée aux widgets locaux.
- Traiter le contenu des widgets comme potentiellement non fiable.
- Gérer explicitement les erreurs de navigation, de script et de chargement.
- Mutualiser les environnements WebView2 lorsque cela est compatible avec l'isolation ; ne pas supposer qu'un seul contrôle suffit pour tous les widgets.

---

## 7. Adaptive Clock Engine

L'horloge adaptative est une fonctionnalité prioritaire et doit être construite par étapes, sans rendre le runtime dépendant d'un modèle d'intelligence artificielle.

### Capacités attendues à terme

- Horloge numérique, typographique et personnalisable.
- Gestion de la police, poids, taille, largeur, alignement et espacement.
- Positionnement libre ou automatique.
- Analyse des couleurs, luminosité et contraste du fond d'écran.
- Identification facultative des zones occupées par un sujet.
- Évitement des éléments visuellement importants.
- Adaptation de la couleur et du matériau.
- Effet de profondeur devant ou derrière un sujet lorsque les données de segmentation sont fiables.
- Transitions fluides lorsque le fond ou le layout change.

### Ordre d'implémentation

1. Horloge statique avec personnalisation typographique.
2. Mise à jour de l'heure avec une fréquence adaptée (seconde ou minute selon le style).
3. Layout manuel et sauvegarde de la position.
4. Analyse légère des couleurs et du contraste du fond d'écran.
5. Positionnement adaptatif dans des zones libres.
6. Détection de sujet et estimation de profondeur en fonctionnalité facultative.
7. Masquage partiel de l'horloge derrière un sujet uniquement si un masque fiable est disponible.

### Contraintes

- Analyser le fond d'écran uniquement lorsqu'il change ou sur demande, pas à chaque frame.
- Mettre en cache les résultats d'analyse avec une clé liée au fond d'écran et à ses dimensions.
- La détection de sujet doit être facultative, asynchrone et remplaçable.
- Prévoir un fonctionnement complet sans IA ni GPU haut de gamme.
- Ne jamais faire dépendre l'affichage de l'heure d'un service réseau.
- Respecter les paramètres régionaux, le format 12/24 heures et le fuseau horaire de Windows.

---

## 8. Visual Effects Engine

Le Visual Effects Engine fournit des matériaux et effets réutilisables pour les widgets et surfaces appartenant à WVX.

### Familles de matériaux

- **Frosted Glass :** flou, teinte, opacité, grain subtil et contours.
- **Liquid Glass :** transparence, réfraction, distorsion, reflets et contours lumineux.
- **Mica-like :** matériau teinté et discret s'inspirant du comportement visuel de Mica.
- **Acrylic-like :** matériau translucide et flouté s'inspirant d'Acrylic.
- **Prismatic Glass :** dispersion chromatique et effets de lumière facultatifs.
- **Custom Material :** combinaison paramétrable des effets disponibles.

Les noms « Mica-like » et « Acrylic-like » désignent des matériaux WVX inspirés de ces rendus. Ne pas prétendre remplacer les matériaux natifs de Windows lorsqu'il s'agit d'une surface WVX indépendante.

### Pipeline cible

```text
Widget / WVX Surface
        │
        ▼
  Material Resolver
        │
        ▼
  Effect Graph / Pipeline
        │
        ├── Background or source texture
        ├── Blur / downsample
        ├── Tint and opacity
        ├── Refraction / distortion
        ├── Light / edge treatment
        └── Final composition
        │
        ▼
   GPU Renderer
        │
        ▼
  Composition Backend
```

### Règles de rendu

- Implémenter d'abord un Frosted Glass simple et mesurable.
- Éviter les copies de textures et les passes GPU redondantes.
- Réduire la résolution des buffers de flou lorsque la qualité le permet.
- Mettre en cache les shaders compilés et les ressources réutilisables.
- Paramétrer les effets pour qu'ils puissent être désactivés ou réduits sur batterie et matériel peu performant.
- Prévoir un fallback sans réfraction et un fallback CPU uniquement si son coût est acceptable.
- Ne pas appliquer plusieurs flous lourds au même arrière-plan sans justification visuelle et mesure de performance.
- Ne jamais capturer de manière récursive une surface WVX qui contient déjà le rendu de cette même surface.
- Documenter les limites de capture de l'arrière-plan, les règles de composition et les risques de boucles de capture.

### Shaders HLSL

- Garder les shaders petits, spécialisés et paramétrables.
- Définir clairement les entrées, sorties, formats de texture et espaces de coordonnées.
- Gérer les tailles nulles, les bords de texture et les paramètres hors limites.
- Compiler les shaders pendant le build ou via un pipeline contrôlé ; éviter la compilation répétée à chaque frame.
- Tester le résultat sur GPU intégré et sur GPU discret lorsque disponibles.

---

## 9. Animation Engine

Le moteur d'animation doit être indépendant des widgets et utilisable par toutes les surfaces WVX.

### Fonctionnalités cibles

- Timeline et keyframes.
- Courbes d'interpolation : linear, cubic Bézier, ease-in/out.
- Spring physics avec limites et amortissement.
- Transitions d'opacité, position, échelle, rayon, couleur et paramètres de matériau.
- Événements de début, progression, fin, annulation et interruption.
- Presets réutilisables.
- Respect des préférences de réduction des animations lorsque cela est possible.

### Contraintes

- Préférer les animations prises en charge par le compositor plutôt que des boucles JavaScript permanentes.
- Éviter les allocations mémoire à chaque frame.
- Permettre d'interrompre proprement une animation et de reprendre depuis l'état courant.
- Ne pas animer des propriétés coûteuses lorsqu'une transition simplifiée suffit.
- Les effets de démarrage WVX sont des animations lancées après la connexion ou au démarrage du runtime ; ne pas les présenter comme une modification de l'animation firmware/boot de Windows.

---

## 10. Windows Integration Engine et limites du système

La compatibilité et la stabilité de Windows sont prioritaires sur l'effet visuel.

### Principe fondamental

WVX peut contrôler librement ses propres fenêtres, widgets, shaders et surfaces. Il ne dispose pas d'une API publique universelle permettant de remplacer arbitrairement le rendu de toutes les fenêtres tierces, de la barre des tâches, du menu Démarrer ou de l'écran de verrouillage.

### Stratégie

- Utiliser les API publiques documentées de Windows.
- Détecter les fonctionnalités disponibles au lieu de supposer qu'elles existent.
- Utiliser les matériaux natifs de Windows lorsqu'une application WVX peut les prendre en charge.
- Pour les surfaces tierces, proposer uniquement les intégrations officiellement supportées et réversibles.
- Documenter séparément les effets WVX natifs, les overlays WVX et les réglages Windows compatibles.
- Prévoir un mode de compatibilité et désactiver une intégration si elle provoque un comportement instable.

### Interdictions techniques

Ne pas utiliser, sauf décision explicite, documentée et assortie d'une stratégie de maintenance et de restauration :

- injection de DLL dans des processus tiers ;
- hooks ou patchs du DWM ;
- modification de binaires Windows ;
- remplacement non pris en charge de composants du shell ;
- détournement d'interfaces internes non documentées ;
- modification non réversible du registre ;
- contournement de l'écran de connexion ou des protections de session.

### Bureau, barre des tâches et écran de verrouillage

- Les widgets de bureau WVX doivent être des fenêtres/surfaces gérées par WVX. Leur placement dans la hiérarchie du bureau doit s'appuyer sur des mécanismes documentés et être testé sur les versions ciblées.
- Ne pas supposer qu'une fenêtre transparente peut automatiquement se placer derrière les icônes du bureau.
- Traiter la barre des tâches et le menu Démarrer comme des intégrations limitées, dépendantes de Windows et non garanties.
- Distinguer l'écran de veille (`.scr` / économiseur d'écran) de l'écran de verrouillage sécurisé. Une personnalisation de l'écran de veille ne donne pas le droit de remplacer l'écran de connexion.
- Toute intégration doit pouvoir être désactivée et revenir à l'état précédent.

---

## 11. WVX Studio

WVX Studio est l'application de configuration et d'édition. Elle ne doit pas héberger le rendu des widgets si le runtime doit continuer à fonctionner lorsque Studio est fermé.

### Sections prévues

- Dashboard : état du runtime et consommation des ressources.
- Widget Library : installer, activer, désactiver, organiser et supprimer les widgets.
- Adaptive Clock : personnaliser les horloges et leur comportement.
- Visual Effects : choisir les matériaux et ajuster les paramètres.
- Windows Appearance : afficher les intégrations disponibles et leur compatibilité.
- Animation Studio : gérer les transitions et presets.
- Theme Editor : créer, sauvegarder, importer et exporter des thèmes.
- Settings : démarrage, batterie, performances, sécurité, logs et mises à jour.

### Règles d'interface

- Séparer clairement les vues, composants, services, modèles et accès IPC.
- Ne pas placer de logique métier complexe dans les composants visuels.
- Prévoir un aperçu en direct, mais limiter sa fréquence et sa consommation.
- Rendre les paramètres avancés accessibles sans surcharger l'écran principal.
- Gérer les états de chargement, erreur, indisponibilité et absence de widgets.
- Ne jamais afficher un effet comme « disponible » si le système a signalé qu'il ne l'est pas.

---

## 12. IPC, configuration et données

### Communication Studio / Runtime

- Définir des messages versionnés et documentés.
- Utiliser un transport local adapté à Windows, par exemple Named Pipes, derrière une abstraction `IpcTransport`.
- Valider taille, type, version et contenu de chaque message.
- Appliquer les contrôles d'accès appropriés au canal local.
- Gérer les déconnexions, délais d'attente, réponses invalides et redémarrages du runtime.
- Ne pas transmettre de pointeurs ou d'objets mémoire bruts entre processus.

### Configuration

- Stocker les préférences utilisateur dans l'emplacement de données applicatives approprié, pas dans le dossier d'installation.
- Versionner chaque format JSON.
- Valider les fichiers avant utilisation.
- Prévoir des migrations explicites entre versions.
- Sauvegarder de manière atomique afin de limiter les risques de corruption.
- Fournir des valeurs par défaut sûres lorsque la configuration est absente ou invalide.
- Ne pas enregistrer de données personnelles ou de secrets dans les logs.

### Thèmes

Un thème peut regrouper :

- tokens de couleurs et typographie ;
- paramètres de matériaux ;
- presets d'animation ;
- réglages par défaut des widgets ;
- règles d'adaptation au fond d'écran.

Un thème ne doit pas contenir de code arbitraire exécuté avec des privilèges natifs.

---

## 13. Performance, mémoire et énergie

La faible consommation est une exigence produit centrale. Les valeurs de performance sont des objectifs de mesure, pas des garanties annoncées avant benchmark.

### Principes obligatoires

- Rendu événementiel : ne pas redessiner lorsqu'aucun changement visuel ne le nécessite.
- Réduire la fréquence des widgets statiques.
- Suspendre le travail des widgets masqués lorsque possible.
- Éviter les allocations et désallocations répétées dans les boucles de rendu.
- Réutiliser les textures, buffers et ressources GPU.
- Contrôler le nombre de WebView2 et leur cycle de vie.
- Limiter les captures d'arrière-plan et les passes de shader.
- Réduire la qualité des effets en mode batterie ou sous forte charge.
- Respecter les paramètres Windows liés à l'économie d'énergie et aux animations.
- Mesurer les temps CPU/GPU, la mémoire privée, la mémoire GPU et le temps de démarrage.

### Modes de qualité

- `Performance` : flou et transparence simples, peu de passes, animations limitées.
- `Balanced` : effets intermédiaires, fréquence adaptative.
- `VisualQuality` : réfraction et effets lumineux lorsque le matériel le permet.

Le mode sélectionné ne doit jamais conduire à dépasser des limites non contrôlées ou à bloquer l'interface.

### Mesures initiales à collecter

- RAM du runtime natif au repos.
- RAM des processus WebView2 séparément.
- CPU au repos et pendant l'animation.
- Utilisation GPU et mémoire vidéo.
- Temps de démarrage à froid et à chaud.
- Nombre de widgets et de surfaces actives.
- Coût par widget et par matériau.

Ne pas inscrire de chiffre de consommation comme fait établi sans mesure reproductible sur une machine de référence documentée.

---

## 14. Qualité du code et conventions

### C++

- Utiliser RAII pour toutes les ressources.
- Préférer les smart pointers et les types explicites.
- Utiliser les wrappers sûrs pour les ressources COM et GPU.
- Éviter les singletons globaux ; privilégier l'injection de dépendances ou des services à durée de vie contrôlée.
- Définir les contrats avec des interfaces lorsque cela simplifie les tests ou le remplacement des backends.
- Limiter les exceptions aux frontières prévues du programme et traduire les erreurs en résultats explicites.
- Éviter les allocations dans les chemins critiques de rendu.
- Isoler les appels Win32 et COM dans des adaptateurs dédiés.
- Utiliser `const` et les types immuables lorsque possible.
- Documenter les règles de thread-affinity des objets COM, UI et GPU.

### C# / WinUI 3

- Respecter le modèle async de WinUI et le thread UI.
- Garder les ViewModels séparés des vues.
- Annuler les opérations asynchrones lorsque les vues ou services sont détruits.
- Ne pas bloquer le thread UI avec des opérations de disque, IPC ou analyse d'image.
- Centraliser les appels IPC dans des services testables.

### TypeScript / CSS

- Activer le mode TypeScript strict si le projet le permet.
- Éviter `any` ; définir des types pour les messages et l'API WVX.
- Séparer état, logique, rendu et styles.
- Nettoyer les timers, subscriptions et listeners à la destruction du widget.
- Éviter les animations infinies non nécessaires.
- Prévoir un rendu utile même si l'API WVX n'est pas disponible.

### Nommage

- Types et classes : `PascalCase`.
- Fonctions et variables : convention existante ; sinon `camelCase`.
- Constantes : convention existante ; sinon `kPascalCase` en C++ ou `UPPER_SNAKE_CASE` selon le contexte.
- Interfaces : préfixe `I` uniquement si cette convention est déjà adoptée dans le module.
- Noms explicites, sans abréviations obscures.

### Organisation

- Un fichier doit avoir une responsabilité identifiable.
- Éviter les classes géantes et les dépendances circulaires.
- Ne pas exposer des détails d'implémentation dans les headers publics.
- Garder les API publiques minimales et documentées.
- Préférer une petite abstraction justifiée à une architecture générique prématurée.

---

## 15. Tests et critères d'acceptation

Toute fonctionnalité doit avoir des critères de validation adaptés.

### Tests attendus

- Tests unitaires pour configuration, validation de manifeste, calculs de layout, transitions et règles de permissions.
- Tests d'intégration pour le démarrage/arrêt du runtime, IPC et cycle de vie des widgets.
- Tests de rendu pour vérifier les matériaux et leurs fallbacks.
- Tests multi-écrans, DPI et changement de résolution pour les fenêtres de widgets.
- Tests de reprise après erreur d'un widget ou déconnexion de Studio.
- Tests de performance au repos et pendant une animation représentative.

### Définition de « terminé »

Une tâche est terminée seulement si :

1. le comportement demandé est implémenté ;
2. les erreurs et cas limites importants sont traités ;
3. les ressources sont libérées correctement ;
4. les tests pertinents passent ;
5. la compilation concernée passe ;
6. la documentation utile est mise à jour ;
7. les limites connues sont indiquées.

Si une vérification ne peut pas être exécutée dans l'environnement disponible, le signaler explicitement sans prétendre qu'elle a réussi.

---

## 16. Roadmap de réalisation

Respecter cet ordre général, sauf décision explicite motivée par le propriétaire du projet.

### Phase 1 — WVX Core

- Structure CMake et cible native minimale.
- Initialisation/arrêt propre du runtime.
- Logging et gestion des erreurs.
- Configuration JSON versionnée.
- Fenêtre native transparente de démonstration.
- Gestion DPI et multi-écrans de base.
- Première communication IPC avec un client de test.

**Livrable :** runtime natif stable affichant une surface transparente contrôlable.

### Phase 2 — Widget Engine minimal

- Manifeste et validation.
- Registre de widgets.
- Gestion du cycle de vie.
- Un widget natif de démonstration.
- Un widget HTML/CSS/TypeScript via WebView2.
- Déplacement, redimensionnement et sauvegarde de position.

**Livrable :** widgets installables et pilotables depuis le runtime.

### Phase 3 — Adaptive Clock

- Horloge numérique et typographique.
- Paramètres de style.
- Configuration par instance.
- Contraste et analyse de couleurs du wallpaper.
- Positionnement adaptatif simple.

**Livrable :** horloge personnalisable et capable de s'adapter à la composition générale du fond.

### Phase 4 — Frosted Glass et matériaux

- Pipeline de matériau.
- Flou GPU.
- Teinte, opacité, bord et bruit subtil.
- Cache et fallback.
- Presets de matériaux.

**Livrable :** Frosted Glass réutilisable dans tous les widgets WVX.

### Phase 5 — WVX Studio

- Dashboard.
- Gestion des widgets.
- Éditeur d'horloge.
- Contrôle des matériaux.
- Communication IPC complète.

**Livrable :** configuration du runtime sans modifier les fichiers manuellement.

### Phase 6 — Effets avancés et intégrations

- Réfraction et distorsion.
- Dispersion lumineuse.
- Animation Engine avancé.
- Économiseur d'écran WVX.
- Étude des intégrations Windows compatibles.

**Livrable :** expérience visuelle étendue avec compatibilité documentée.

### Phase 7 — Stabilisation et distribution

- Profils de performance.
- Tests de longue durée.
- Packaging et désinstallation propre.
- Migrations de configuration.
- Documentation utilisateur et développeur.

**Livrable :** version distribuable, réversible et maintenable.

---

## 17. Première tâche recommandée lorsqu'on démarre sur un dépôt vide

Si le dépôt ne contient pas encore de code, ne pas générer immédiatement toute l'architecture cible.

Commencer par :

1. vérifier les outils installés et les versions disponibles (Visual Studio/MSVC, CMake, Windows SDK, .NET SDK) ;
2. créer un squelette minimal CMake pour `wvx-runtime` ;
3. créer une application native qui démarre, affiche une fenêtre de démonstration et se ferme proprement ;
4. ajouter un système de logs et de gestion d'erreurs minimal ;
5. ajouter les tests de base et documenter les commandes de build ;
6. vérifier la compilation avant d'ajouter le Widget Engine.

Si le dépôt contient déjà du code, adapter ce plan à son état réel et préserver les composants existants.

### Commandes de vérification

N'exécuter que les commandes correspondant aux fichiers de build réellement présents. Pour un projet CMake, vérifier notamment la configuration, la compilation et les tests CTest. Pour Studio, vérifier séparément le build .NET. Documenter les commandes exactes dans le `README.md` du dépôt lorsqu'elles sont confirmées.

---

## 18. Documentation de référence

Privilégier les documentations officielles et la version correspondant au SDK réellement utilisé :

- Windows API / Win32 : <https://learn.microsoft.com/windows/win32/>
- Direct2D : <https://learn.microsoft.com/windows/win32/direct2d/direct2d-portal>
- Direct3D 11 : <https://learn.microsoft.com/windows/win32/direct3d11/overviews-direct3d-11-devices-intro>
- DirectComposition : <https://learn.microsoft.com/windows/win32/directcomp/directcomposition-portal>
- Windows App SDK / WinUI 3 : <https://learn.microsoft.com/windows/apps/winui/winui3/>
- WebView2 : <https://learn.microsoft.com/microsoft-edge/webview2/>
- WebView2 performance : <https://learn.microsoft.com/microsoft-edge/webview2/concepts/performance>
- GitHub Copilot repository instructions : <https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions>

Ne pas se fier à un exemple ancien sans vérifier qu'il correspond aux versions actuelles des API, du SDK et du compilateur du projet.

---

## 19. Format des réponses de Copilot pendant le développement

Pour chaque tâche technique non triviale, structurer le compte rendu ainsi :

1. **Compréhension :** objectif et périmètre.
2. **Inspection :** fichiers et composants existants concernés.
3. **Plan :** changements ciblés.
4. **Implémentation :** fichiers créés ou modifiés et raison.
5. **Validation :** commandes réellement exécutées et résultats.
6. **Limites :** points non testés, API indisponibles ou travaux restant à faire.

Rester précis. Ne pas noyer les réponses dans des explications génériques et ne pas présenter une proposition comme une fonctionnalité déjà réalisée.

**Règle finale : construire WVX par incréments fiables. La stabilité, la compatibilité Windows et l'efficacité des ressources priment sur l'ajout rapide d'effets visuels.**
