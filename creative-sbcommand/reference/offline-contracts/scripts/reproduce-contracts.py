"""Reproduce static range-construction evidence without executing vendor code."""
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile

project = Path(__file__).resolve().parents[3]
platform = Path("C:/Program Files (x86)/Creative/Sound Blaster Command/Platform")
dotnet = Path("C:/Program Files/dotnet/dotnet.exe")
cli = project / "tools/ilspycmd/tools/net8.0/any/ilspycmd.dll"
evidence = project / "reference/offline-contracts"
sources = []
lines = []
with tempfile.TemporaryDirectory(prefix=".contract-repro-", dir=project / "tools") as temporary:
    temporary = Path(temporary)
    for name, folder in [("Creative.Platform.Devices.dll", "devices"),
                         ("Creative.Platform.Mixer.dll", "mixer"),
                         ("Creative.SBConnect.UI.Framework.dll", "uifw")]:
        source = platform / name
        output = temporary / folder
        command = [str(dotnet), "--roll-forward", "Major", str(cli),
                   "--disable-updatecheck", "-p", "-o", str(output), str(source)]
        subprocess.run(command, check=True, capture_output=True, timeout=120)
        sources.append({"path": str(source), "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                        "command": command})
        for file in sorted(output.rglob("*.cs")):
            for number, line in enumerate(file.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if "new EffectParameterRangeConfig<" in line:
                    lines.append(f"{folder}/{file.relative_to(output).as_posix()}:{number}: {line.strip()}")
(evidence / "reproduced-ranges.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
(evidence / "reproduction-sources.json").write_text(json.dumps(sources, ensure_ascii=False, indent=2), encoding="utf-8")
print(f"Static reproduction complete: {len(lines)} range-construction source lines; temporary trees removed.")
