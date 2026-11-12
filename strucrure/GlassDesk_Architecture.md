# GlassDesk — Architecture historique

> **Archive de l'ancien prototype Rust.** Le projet migre désormais vers WVX,
> C++20/CMake pour le runtime et C#/WinUI 3 pour Studio. Pour les décisions
> actives, consulter `copilot-instructions.md`. Les recommandations Rust et
> `wgpu` ci-dessous ne décrivent plus la stack cible.

> Moteur de widgets, de style et d'effets visuels pour Windows 11
> Objectifs : **fluide · ultra léger (RAM/CPU) · très personnalisable**
> Styles visés : **One UI (frosted glass)** et **Liquid Glass (iOS)**

---

## 1. Objectifs et priorités

| Priorité | Objectif | Cible mesurable |
|---|---|---|
| 1 | Widgets personnalisés très rapides | < 5 ms de rendu par frame, 60 fps (120 fps si écran compatible) |
| 2 | Frosted glass sur tout Windows 11 (fenêtres, bureau, barre des tâches) | Effet cohérent, désactivable par application |
| 3 | Widget horloge adaptatif au fond d'écran | Analyse unique à chaque changement de fond, coût nul ensuite |
| 4 | Animations et effets personnalisés (allumage d'écran, écran de veille) | Animations pilotées par le GPU |
| 5 | Légèreté | **< 40 Mo de RAM** au repos avec 5 widgets, **< 1 % CPU** au repos |

---

## 2. Choix technologiques (et pourquoi)

### 2.1 Langage du cœur : **Rust**

- Pas de runtime ni de ramasseur de miettes : RAM faible et prévisible.
- Binaire natif unique (quelques Mo), démarrage instantané.
- Accès complet à Win32, DWM, DirectComposition et Direct3D via la crate `windows`.
- Rendu GPU avec `wgpu` (DirectX 12/11) et shaders WGSL/HLSL.

> Un cœur en C# ou Electron coûterait 100 à 300 Mo de RAM. Rust divise ce chiffre par 5 à 10.

### 2.2 Langage des widgets : **un CSS-like natif + un mini langage d'expressions**

Il faut être honnête sur « CSS = le plus rapide » : le CSS est très expressif pour les effets visuels, mais **un moteur de navigateur complet est lourd** (WebView2 = 50 à 100 Mo par groupe de processus). La meilleure solution est donc **deux niveaux** :

| Niveau | Technologie | Usage | Coût |
|---|---|---|---|
| **Tier 1 — Widgets natifs** (par défaut) | Format `.gdw` : **CSS-subset** (parsé avec `lightningcss`) + expressions + shaders | Horloge, météo, monitoring, musique, calendrier… | ~1-3 Mo par widget |
| **Tier 2 — Widgets Web** (optionnel) | HTML/CSS/JS dans **WebView2 partagé** (un seul environnement, un seul processus navigateur pour tous les widgets Web) | Widgets complexes, interactifs, ou portés depuis le web | ~40-80 Mo partagés |

Le Tier 1 est la voie recommandée : on garde la syntaxe CSS (connue de tous, très permissive pour les effets) mais on la rend directement sur le GPU, sans navigateur.

### 2.3 Pourquoi ne pas faire des widgets en pur CSS dans un navigateur ?

`backdrop-filter: blur()` en CSS **ne floute pas le bureau** derrière une fenêtre transparente : il floute seulement le contenu de la page. Pour obtenir un vrai verre dépoli sur le bureau, le flou doit être fait **côté système** (DWM) ou **côté moteur natif** (capture + shader). Notre moteur natif règle ce problème proprement.

---

## 3. Architecture des processus

```
┌────────────────────────────────────────────────────────────┐
│          GlassDesk Studio (interface de réglages)          │
│   Lancée à la demande, fermée ensuite → 0 Mo au repos      │
│   Éditeur de thèmes · Galerie · Éditeur de widgets         │
└──────────────────────┬─────────────────────────────────────┘
                       │ Named Pipe (IPC léger, JSON/MessagePack)
┌──────────────────────▼─────────────────────────────────────┐
│               GlassDesk Core  (1 seul processus)           │
│                                                            │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Widget Host │  │ Style Engine │  │ Animation Engine  │  │
│  └──────┬──────┘  └──────┬───────┘  └─────────┬─────────┘  │
│         └────────────────▼────────────────────┘            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Render Engine (wgpu / D3D12, 1 seul device partagé) │  │
│  │  Blur · Réfraction · Dispersion · Spéculaire · Grain │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌───────────────┐ ┌───────────────┐ ┌──────────────────┐  │
│  │ Shell Styler  │ │ Wallpaper     │ │ Power & Session  │  │
│  │ (fenêtres,    │ │ Analyzer      │ │ (écran, veille,  │  │
│  │  taskbar…)    │ │ (IA, 1 fois)  │ │  déverrouillage) │  │
│  └───────────────┘ └───────────────┘ └──────────────────┘  │
└──────────────────────┬─────────────────────────────────────┘
                       │ (optionnel, activable par l'utilisateur)
┌──────────────────────▼─────────────────────────────────────┐
│  GlassDesk Hook (DLL légère injectée : Explorer, taskbar)  │
│  Isolée, désactivable, jamais chargée par défaut           │
└────────────────────────────────────────────────────────────┘
```

### Règles d'économie de ressources

1. **Un seul processus** pour tous les widgets Tier 1 (pas un processus par widget).
2. **Un seul device GPU** partagé et un seul pipeline de composition.
3. **Rendu à la demande** : un widget n'est redessiné que si une donnée change (une horloge se redessine 1×/s, sauf animations des secondes).
4. **Surfaces mises en cache** : le flou et les ombres statiques sont calculés une fois, puis réutilisés.
5. **Pause intelligente** : suspension automatique si jeu plein écran, session verrouillée, écran éteint ou mode économie d'énergie.
6. **Studio fermé = 0 Mo** : l'interface de configuration n'est pas résidente.
7. **Flou en basse résolution** : downsampling (1/2 à 1/4) puis flou gaussien/Kawase, puis upsampling. Qualité identique à l'œil, coût divisé par 4 à 16.

---

## 4. Arborescence du projet

```
glassdesk/
├── Cargo.toml                      # Workspace Rust
├── crates/
│   ├── core/                       # Processus principal (service + tray)
│   │   ├── main.rs
│   │   ├── ipc.rs                  # Named pipe vers Studio
│   │   ├── power.rs                # Écran, veille, batterie, plein écran
│   │   └── config.rs               # Chargement/validation JSON
│   │
│   ├── widget-host/                # Hébergement des widgets
│   │   ├── window.rs               # Fenêtres sans bordure + DirectComposition
│   │   ├── desktop_attach.rs       # Accrochage WorkerW (derrière les fenêtres)
│   │   ├── layout.rs               # Grille, magnétisme, multi-écrans, DPI
│   │   ├── lifecycle.rs            # Chargement/déchargement à chaud
│   │   └── webview_tier.rs         # Tier 2 : WebView2 partagé
│   │
│   ├── gdw-lang/                   # Langage des widgets
│   │   ├── parser_css.rs           # CSS-subset (lightningcss)
│   │   ├── layout_flex.rs          # Flexbox/Grid (taffy)
│   │   ├── expr.rs                 # Expressions : {{time.hour}}, calc(), clamp()
│   │   ├── bindings.rs             # Données live → propriétés
│   │   └── shader_props.rs         # Propriétés CSS spéciales (glass-*, light-*)
│   │
│   ├── render/                     # Moteur de rendu GPU
│   │   ├── device.rs               # wgpu : 1 device partagé
│   │   ├── backdrop.rs             # Capture du fond (Windows.Graphics.Capture)
│   │   ├── text.rs                 # Texte vectoriel (cosmic-text), polices variables
│   │   ├── vector.rs               # Formes, squircles (vello ou lyon)
│   │   ├── cache.rs                # Cache de textures
│   │   └── shaders/
│   │       ├── blur_kawase.wgsl
│   │       ├── frosted.wgsl         # One UI : flou + teinte + grain
│   │       ├── liquid_glass.wgsl    # Réfraction + dispersion + spéculaire
│   │       ├── squircle_sdf.wgsl    # Coins continus
│   │       ├── edge_light.wgsl      # Éclairage de bord
│   │       └── noise.wgsl
│   │
│   ├── style/                      # Moteur de thèmes
│   │   ├── tokens.rs               # Design tokens
│   │   ├── presets.rs              # One UI, Liquid Glass, Clear, Tinted…
│   │   └── adaptive.rs             # Teinte adaptative selon le fond
│   │
│   ├── anim/                       # Moteur d'animation
│   │   ├── spring.rs               # Ressorts (style iOS)
│   │   ├── timeline.rs             # Séquences et keyframes
│   │   ├── presets.rs              # Ouverture, fermeture, survol, morph
│   │   └── wake_effects.rs         # Animations d'allumage d'écran
│   │
│   ├── shell-styler/               # Style de l'interface Windows
│   │   ├── window_effects.rs       # Mica/Acrylic/coins via DWM
│   │   ├── taskbar.rs              # Transparence/flou de la barre des tâches
│   │   ├── event_hook.rs           # SetWinEventHook (ouverture, déplacement…)
│   │   ├── rules.rs                # Règles par application (liste blanche/noire)
│   │   └── glass_underlay.rs       # Calque de verre sous les fenêtres
│   │
│   ├── wallpaper-ai/               # Analyse du fond d'écran
│   │   ├── watcher.rs              # Détecte le changement de fond
│   │   ├── segmentation.rs         # Masque du sujet (modèle ONNX léger)
│   │   ├── free_space.rs           # Zones libres pour l'horloge
│   │   └── palette.rs              # Couleurs dominantes
│   │
│   ├── screensaver/                # Écran de veille (.scr)
│   │   └── main.rs
│   │
│   └── hook-dll/                   # DLL optionnelle (Explorer, taskbar avancée)
│
├── studio/                         # Interface de configuration
│   └── (Tauri : Rust + WebView2, lancée à la demande)
│
├── themes/
│   ├── oneui-frosted/theme.json
│   ├── liquid-glass/theme.json
│   ├── liquid-glass-clear/theme.json
│   └── minimal/theme.json
│
├── widgets/
│   ├── clock-adaptive/             # Widget vedette (voir §8)
│   │   ├── manifest.json
│   │   ├── widget.gdw
│   │   └── assets/
│   ├── weather/
│   ├── system-monitor/
│   ├── media-player/
│   └── calendar/
│
├── models/
│   └── subject-seg.onnx            # Modèle de segmentation (~5 Mo)
│
├── docs/
└── installer/                      # MSIX / WiX
```

---

## 5. Le format de widget `.gdw` (CSS-subset)

Un widget est un fichier texte : structure, style CSS et données liées. Syntaxe familière, rendu 100 % GPU.

### 5.1 `manifest.json`

```json
{
  "id": "com.exemple.clock-adaptive",
  "name": "Horloge adaptative",
  "version": "1.0.0",
  "entry": "widget.gdw",
  "tier": 1,
  "defaultSize": [420, 220],
  "resizable": true,
  "permissions": ["wallpaper"],
  "data": ["time", "wallpaper"],
  "refresh": "1s"
}
```

### 5.2 `widget.gdw`

```html
<widget>
  <clock class="time">{{time.hh}}:{{time.mm}}</clock>
  <label class="date">{{time.date("dddd d MMMM")}}</label>
</widget>

<style>
  widget {
    width: 420px; height: 220px;
    border-radius: 36px;
    corner-smoothing: 0.65;            /* squircle One UI / iOS */
    material: liquid-glass;            /* ou frosted */
    glass-blur: 24px;
    glass-refraction: 0.35;
    glass-dispersion: 0.04;            /* décomposition de la lumière */
    glass-specular: 0.8;
    glass-tint: adaptive;              /* teinte selon le fond */
    shadow: 0 12px 30px rgba(0,0,0,.18);
    transition: all 420ms spring(220, 24);
  }

  .time {
    font-family: "Inter Variable";
    font-variation-settings: "wght" 600, "wdth" fit(widget.free-width);
    font-size: fit(widget.free-height);
    color: glass-text;
  }

  widget:hover { glass-refraction: 0.5; scale: 1.03; }
</style>
```

### 5.3 Propriétés CSS spéciales (extensions GlassDesk)

| Propriété | Rôle |
|---|---|
| `material` | `frosted` (One UI), `liquid-glass`, `acrylic`, `solid` |
| `glass-blur` | Rayon de flou |
| `glass-refraction` | Intensité de déformation du fond |
| `glass-dispersion` | Séparation chromatique (effet prisme) |
| `glass-specular` | Reflets sur les bords |
| `glass-tint` | Teinte fixe ou `adaptive` |
| `glass-noise` | Grain du verre dépoli |
| `corner-smoothing` | Lissage des coins (squircle) |
| `light-angle` | Direction de la source de lumière |
| `fit()` | Adapte une valeur à l'espace libre du fond |
| `spring(k, d)` | Courbe d'animation à ressort |

### 5.4 Shaders personnalisés

Les créateurs avancés peuvent fournir leur propre shader (WGSL) :

```css
widget { material: custom("my-effect.wgsl"); }
```

Les shaders sont compilés une fois, mis en cache et exécutés dans une limite de temps GPU.

---

## 6. Moteur de style (frosted glass One UI et Liquid Glass)

### 6.1 Design tokens

```json
{
  "name": "One UI Frosted",
  "shape": { "radius": 32, "smoothing": 0.6 },
  "material": {
    "type": "frosted",
    "blur": 40,
    "tintOpacity": 0.55,
    "saturation": 1.4,
    "noise": 0.03
  },
  "shadow": { "blur": 24, "opacity": 0.12, "y": 8 },
  "motion": { "preset": "spring", "stiffness": 260, "damping": 26 }
}
```

```json
{
  "name": "Liquid Glass",
  "shape": { "radius": 28, "smoothing": 0.7 },
  "material": {
    "type": "liquid-glass",
    "blur": 18,
    "refraction": 0.35,
    "dispersion": 0.04,
    "specular": { "intensity": 0.8, "angle": 135 },
    "edgeLight": true,
    "tint": "adaptive"
  }
}
```

### 6.2 Pipeline de rendu du verre (par widget)

1. **Capture du fond** derrière le widget (Windows.Graphics.Capture, zone réduite uniquement).
2. **Downsampling** (1/2 à 1/4).
3. **Flou Kawase** (3 à 5 passes, bien moins coûteux qu'un gaussien).
4. **Frosted (One UI)** : saturation, teinte, grain → composition.
5. **Liquid Glass** : en plus
   - champ de distance au bord (SDF du squircle) → normales,
   - déformation des UV du fond selon la normale (**réfraction**),
   - décalage différent par canal R/G/B (**décomposition de la lumière / dispersion**),
   - reflets spéculaires et éclairage de bord.
6. **Ombre** et composition finale via DirectComposition.

> Optimisation clé : la capture et le flou ne sont **recalculés que si le fond change** (nouvelle fenêtre derrière, vidéo, déplacement). Un fond d'écran statique donne un coût GPU quasi nul.

---

## 7. Style de tout Windows 11 : frosted glass global

Cette partie est la plus délicate. Voici ce qui est **réellement faisable**, avec le niveau de fiabilité de chaque méthode.

| Élément | Méthode | Fiabilité | Remarque |
|---|---|---|---|
| **Barre de titre et fond de fenêtres** (Win32, la plupart des apps) | `DwmSetWindowAttribute` (Mica/Acrylic/Tabbed) | Élevée | API officielle |
| **Barre des tâches** | `SetWindowCompositionAttribute` (méthode TranslucentTB) + style avancé via hook | Élevée à moyenne | Peut casser après une mise à jour |
| **Bureau** | Fond d'écran rendu par GlassDesk dans WorkerW + calque de verre + icônes sur verre | Élevée | Entièrement sous notre contrôle |
| **Explorateur de fichiers (contenu)** | Hook (type Windhawk) pour rendre la zone cliente transparente | Moyenne | Module optionnel isolé |
| **Menu Démarrer, centre de notifications** | Hook ciblé ou style de remplacement | Faible à moyenne | Fragile, à traiter en dernier |
| **Applications au contenu opaque** (Chrome, VS Code, jeux…) | **Calque de verre sous la fenêtre** (voir ci-dessous) : verre sur la bordure et la barre de titre, contenu inchangé | Moyenne | Le contenu opaque reste opaque |

### Calque de verre sous les fenêtres (`glass_underlay`)

Pour les applications dont on ne peut pas modifier l'intérieur, GlassDesk place sous chaque fenêtre un **calque de verre** : cadre, ombre, bordure lumineuse, coins arrondis harmonisés. La fenêtre suit en temps réel grâce à `SetWinEventHook` (déplacement, redimensionnement, minimisation, ouverture).

### Système de règles par application

```json
{
  "rules": [
    { "process": "explorer.exe", "effect": "frosted", "deep": true },
    { "process": "chrome.exe", "effect": "frame-only" },
    { "process": "game*.exe", "effect": "disabled" }
  ]
}
```

### Garde-fous

- Liste noire automatique : jeux, lecteurs vidéo plein écran, applications de sécurité, outils d'administration.
- Désactivation instantanée par raccourci clavier (« mode sûr »).
- Le module `hook-dll` n'est **jamais** chargé sans accord explicite et peut être retiré à chaud.
- Journal d'erreurs et retour automatique au style Windows par défaut en cas de plantage.

---

## 8. Widget vedette : horloge adaptative au fond d'écran

### 8.1 Comportement visé

L'horloge s'adapte aux éléments du fond d'écran, dans l'esprit de l'écran verrouillé iOS :

- **Effet de profondeur** : l'horloge passe *derrière* le sujet de la photo (personne, montagne, bâtiment).
- **Longueur adaptative** : le texte s'étire ou se contracte (largeur, graisse, taille) pour remplir exactement la zone libre disponible.
- **Couleurs automatiques** : teinte dérivée de la palette du fond d'écran, avec contraste garanti.
- **Style personnalisable** : police, graisse, verre, ombre, animation des chiffres.

### 8.2 Pipeline (exécuté une seule fois par fond d'écran)

```
Changement de fond d'écran (détecté par watcher.rs)
        │
        ▼
 1. Redimensionnement de l'image (max 512 px de côté)
        │
        ▼
 2. Segmentation du sujet (ONNX, ~5 Mo, quelques ms)  →  masque du sujet
        │
        ▼
 3. Calcul des zones libres (rectangles sans sujet)   →  zone d'horloge
        │
        ▼
 4. Extraction de la palette + luminosité de la zone  →  couleurs, contraste
        │
        ▼
 5. Mise en cache : mask.png + layout.json + palette.json
        │
        ▼
 Rendu : l'horloge est dessinée, puis le masque est
 superposé par-dessus pour l'effet de profondeur
```

Le modèle d'IA est **chargé, utilisé, puis déchargé** : il ne reste pas en mémoire. Résultat : pas de coût continu.

### 8.3 Adaptation de la longueur du texte

La police est une **police variable** (axes `wght`, `wdth`). Le moteur résout :

```
trouver (font-size, wdth, wght) tels que
    largeur_texte == largeur_zone_libre
    hauteur_texte <= hauteur_zone_libre
    contraste(texte, fond) >= 4.5
```

Résolution par recherche dichotomique (quelques itérations, résultat mis en cache). Recalculée uniquement si l'heure change de format (ex. `9:41` → `10:41`) ou si la zone change.

### 8.4 Options de style de l'horloge

| Option | Valeurs |
|---|---|
| Matière | Liquid Glass, Frosted, Plein, Néon, Dégradé |
| Mode d'adaptation | Remplir la zone, Garder les proportions, Taille fixe |
| Profondeur | Activée/désactivée, opacité du sujet |
| Police | Toute police variable (fournie ou installée) |
| Format | 12 h/24 h, secondes, date, jour |
| Animation | Roulement des chiffres, morph, fondu, ressort |
| Couleurs | Auto (palette du fond), fixe, dégradé |

### 8.5 Affichage sur d'autres surfaces

- **Bureau** : widget classique (WorkerW), avec effet de profondeur derrière les icônes.
- **Écran de veille** : voir §9.
- **Écran de verrouillage Windows** : *non modifiable* par une application classique (voir limites, §12).

---

## 9. Animations et effets système

### 9.1 Animation d'allumage de l'écran

Détection via `RegisterPowerSettingNotification` (`GUID_CONSOLE_DISPLAY_STATE`) et les notifications de session (`WTSRegisterSessionNotification`).

Au réveil de l'écran ou au déverrouillage, GlassDesk affiche une **surface plein écran temporaire** (topmost, transparente aux clics) qui joue une animation puis disparaît :

| Preset | Description |
|---|---|
| **Fade liquide** | Le bureau apparaît à travers une vague de verre |
| **Ripple** | Onde circulaire depuis le centre ou le curseur |
| **CRT / Power-on** | Ligne lumineuse qui s'ouvre en image |
| **Iris** | Ouverture circulaire |
| **Bloom** | Lumière douce qui monte et se stabilise |
| **Personnalisé** | Shader fourni par l'utilisateur |

Durée courte (300 à 800 ms), 0 % CPU hors animation, la surface est détruite ensuite.

### 9.2 Animations des fenêtres

- `SetWinEventHook` capte ouverture, fermeture, minimisation, restauration, déplacement.
- `DwmRegisterThumbnail` / capture permet de dessiner une **copie animée** de la fenêtre (zoom ressort, flou de mouvement, effet « goutte » Liquid Glass) pendant que la vraie fenêtre est masquée ou apparaît.

### 9.3 Moteur d'animation

- Ressorts physiques (raideur, amortissement, masse) pour un rendu naturel.
- Timelines à keyframes pour les séquences.
- Toutes les valeurs animées vivent sur le GPU, le CPU n'envoie que les cibles.
- Mode économie : réduit la fréquence ou supprime les effets sur batterie.

---

## 10. Écran de veille personnalisé (horloge Liquid Glass)

- Format `.scr` (écran de veille Windows standard), écrit en Rust, utilise le même moteur de rendu que les widgets.
- Réutilise le widget horloge adaptatif (§8) en plein écran : verre liquide animé, fond d'écran ou dégradé en mouvement, léger déplacement anti-brûlure d'écran.
- Aperçu et configuration depuis le Studio.
- S'installe via le mécanisme natif de Windows (Paramètres → Écran de veille).

---

## 11. Studio (interface de configuration)

Application **à la demande** (Tauri : Rust + WebView2), qui se ferme après usage.

- **Galerie** de widgets : glisser-déposer sur le bureau.
- **Éditeur de thème en direct** : curseurs de flou, réfraction, dispersion, rayon, teinte, vitesse d'animation, aperçu immédiat.
- **Éditeur de widget** : code `.gdw` avec rechargement à chaud (hot reload), console d'erreurs.
- **Éditeur d'effets système** : règles par application, intensité globale de verre.
- **Profils** : Travail, Gaming, Minimal, Économie d'énergie, avec bascule automatique.
- **Import/export** : packs `.gdpack` (thème + widgets + animations).

---

## 12. Limites à connaître

| Sujet | Réalité |
|---|---|
| **Écran de verrouillage Windows** | Tourne sur un bureau sécurisé : impossible de le modifier proprement. Solution : écran de veille personnalisé + animation au déverrouillage. |
| **Contenu des applications opaques** | Une application qui dessine un fond opaque ne peut pas devenir transparente sans la modifier. On stylise le cadre, pas l'intérieur. |
| **Menu Démarrer, Paramètres, centre de notifications** | Dépendent de composants internes de Windows (XAML/Shell). Modifiables seulement par hook fragile. |
| **API non documentées** | Peuvent casser après une mise à jour Windows : à isoler dans des modules désactivables. |
| **`backdrop-filter` CSS en navigateur** | Ne floute pas le bureau : d'où le choix d'un moteur natif pour le Tier 1. |
| **Droits d'auteur** | Reproduire le *style* est permis ; ne pas utiliser les polices (SF Pro, One UI Sans) ni les icônes officielles. Utiliser Inter, Roboto Flex et des icônes libres. |

---

## 13. Budget de performance cible

| Scénario | RAM | CPU | GPU |
|---|---|---|---|
| Au repos, 0 widget | ~12 Mo | ~0 % | 0 % |
| 5 widgets Tier 1 (fond statique) | ~30-40 Mo | < 1 % | < 2 % |
| Horloge adaptative + verre | +5 Mo | < 0,5 % | < 1 % |
| Frosted global (fenêtres) | +10-20 Mo | < 1 % | 2-5 % |
| Animation d'allumage (800 ms) | +5 Mo (temporaire) | pic bref | pic bref |
| Studio ouvert | +80-120 Mo (temporaire) | variable | faible |

Mesures à intégrer dès le début : profilage GPU (PIX, RenderDoc), compteur interne de frame time, test de régression de RAM dans la CI.

---

## 14. Feuille de route

| Phase | Contenu | Durée estimée |
|---|---|---|
| **0. Prototype** | Fenêtre transparente + accrochage au bureau (WorkerW) + rendu wgpu + flou Kawase | 2-3 sem. |
| **1. Widgets Tier 1** | Parser `.gdw` (CSS-subset), layout flex, texte, liaison de données, hot reload, 2 widgets (horloge, météo) | 4-5 sem. |
| **2. Matériaux** | Frosted One UI + squircles + thèmes JSON + ombres | 3 sem. |
| **3. Horloge adaptative** | Watcher de fond, segmentation ONNX, zones libres, `fit()`, effet de profondeur | 3-4 sem. |
| **4. Liquid Glass** | Réfraction, dispersion, spéculaire, éclairage de bord | 4 sem. |
| **5. Frosted global** | DWM sur fenêtres, barre des tâches, calque sous fenêtres, règles par app | 4-6 sem. |
| **6. Animations** | Moteur à ressort, animation d'allumage, animations de fenêtres | 3-4 sem. |
| **7. Écran de veille** | `.scr` avec horloge Liquid Glass | 2 sem. |
| **8. Studio** | Galerie, éditeur de thèmes/widgets, profils, `.gdpack` | 4 sem. |
| **9. Tier 2 + SDK** | WebView2 partagé, API publique, sandbox, documentation | 3-4 sem. |
| **10. Finition** | Optimisation, installateur MSIX, tests de stabilité | 3 sem. |

**Périmètre actuel (objectif immédiat)** : phases 0 → 3 + 5 (partie fenêtres/barre des tâches/bureau), c'est-à-dire **widgets personnalisés + frosted glass global de Windows 11**, avec l'horloge adaptative comme widget phare.

---

## 15. Prochaines étapes concrètes

1. Initialiser le workspace Rust (`cargo new --workspace`) avec les crates `core`, `widget-host`, `render`.
2. Créer une fenêtre widget transparente accrochée au bureau (WorkerW) avec DirectComposition.
3. Implémenter le flou Kawase sur la capture du fond (premier effet visible).
4. Écrire le parseur `.gdw` minimal (rectangle, texte, `border-radius`, `material: frosted`).
5. Construire le widget horloge simple, puis ajouter l'analyse du fond d'écran.

---

## 16. Dépendances principales (Rust)

| Crate | Usage |
|---|---|
| `windows` | API Win32, DWM, DirectComposition, Graphics.Capture |
| `wgpu` | Rendu GPU (DirectX) et shaders |
| `lightningcss` | Parsing CSS |
| `taffy` | Layout flexbox/grid |
| `cosmic-text` | Texte et polices variables |
| `vello` ou `lyon` | Formes vectorielles |
| `ort` (ONNX Runtime) | Segmentation du fond d'écran |
| `serde` / `serde_json` | Configuration et thèmes |
| `tauri` | Studio de configuration |
| `notify` | Surveillance des fichiers (hot reload) |
