from pathlib import Path
from typing import Optional
import json

# 설정 파일 기본 경로
DEFAULT_CONFIG_PATH = Path("config.json")


def load_config(path: Optional[Path] = None) -> dict:
    """
    설정 파일을 불러오는 함수입니다.
    파일이 없으면 빈 딕셔너리를 반환합니다.
    """
    # 경로가 지정되지 않으면 기본값 사용
    config_path = path or DEFAULT_CONFIG_PATH
    if not config_path.exists():
        return {}

    error_msg = "설정 파일 형식이 올바르지 않습니다"
    try:
        with open(config_path) as f:
            return json.load(f)
    except json.JSONDecodeError:
        # JSON 파싱 실패 시 오류 출력
        print(error_msg)
        return {}
