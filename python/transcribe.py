"""Small JSON-only local transcription entrypoint used by Edentic."""

from __future__ import annotations

import json
import sys


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if len(sys.argv) != 4:
        fail("Usage: transcribe.py <media> <model> <compute-mode>")

    media_path, model_name, compute_mode = sys.argv[1:]
    try:
        import ctranslate2
        from faster_whisper import WhisperModel
    except ImportError:
        fail(
            "Local transcription is not installed. Run: "
            "py -3.12 -m pip install -r python\\requirements.txt"
        )

    cuda_available = ctranslate2.get_cuda_device_count() > 0
    if compute_mode == "cpu":
        device, compute_type = "cpu", "int8"
    elif compute_mode == "gpu":
        if not cuda_available:
            fail("GPU preferred was selected, but CTranslate2 could not access a CUDA device.")
        device, compute_type = "cuda", "float16"
    elif compute_mode == "hybrid":
        device, compute_type = ("cuda", "float16") if cuda_available else ("cpu", "int8")
    else:
        device, compute_type = ("cuda", "float16") if cuda_available else ("cpu", "int8")

    model = WhisperModel(model_name, device=device, compute_type=compute_type)
    segments, _ = model.transcribe(
        media_path,
        vad_filter=True,
        beam_size=5,
        condition_on_previous_text=True,
    )
    payload = [
        {"start": round(segment.start, 3), "end": round(segment.end, 3), "text": segment.text.strip()}
        for segment in segments
        if segment.text.strip()
    ]
    json.dump(payload, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()

