# WVX (migration en cours)

WVX est une application native Windows 11 de widgets et de personnalisation
visuelle. Ce dépôt migre le prototype GlassDesk/Rust vers le runtime C++20 /
CMake et le Studio C# / WinUI 3 décrits dans
[`strucrure/copilot-instructions.md`](strucrure/copilot-instructions.md).

## État actuel

- Runtime Win32 natif avec une horloge Direct2D et un matériau fourni par le
  backdrop système Windows 11 lorsque disponible.
- Paramètres JSON versionnés, validés et écrits atomiquement dans
  `%LOCALAPPDATA%\WVX\settings.json`; l'ancien fichier GlassDesk est lu comme
  source de migration.
- Named Pipe local `wvx-runtime-v1` pour lire et appliquer les réglages depuis
  Studio ; le contrat est défini dans [`docs/ipc.md`](docs/ipc.md).
- L'horloge est le seul widget actuellement hébergé. Les autres cartes,
  l'analyse du fond d'écran, les shaders GPU personnalisés et les intégrations
  Windows globales restent hors de ce MVP.
- Le prototype Rust historique demeure dans `crates/` jusqu'à validation du
  remplacement natif ; ne pas l'utiliser comme cible des nouvelles
  fonctionnalités WVX.

## Prérequis

- Windows 11 x64 et Windows SDK.
- Visual Studio 2022 avec MSVC et C++ CMake tools.
- CMake 3.25 ou supérieur.
- .NET 8 SDK et le workload Windows App SDK / WinUI 3 pour le Studio.

## Construire et tester le runtime

```powershell
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```

Le runtime se trouve dans `build\Debug\wvx-runtime.exe` avec le générateur
Visual Studio. Lance le runtime avant WVX Studio pour que le pipe de
configuration soit disponible.

## Construire le Studio

```powershell
dotnet build apps\wvx-studio\WvxStudio.csproj
```

Le Studio signale explicitement l'indisponibilité du runtime si le pipe est
absent. Il ne sauvegarde pas les réglages dans un fichier de substitution.
