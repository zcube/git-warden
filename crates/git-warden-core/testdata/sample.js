import path from 'path';
import { readFileSync } from 'fs';
import 'dotenv/config';

// 설정 파일을 불러오는 함수
function loadConfig(filePath) {
  // 파일 내용을 읽어서 JSON으로 파싱합니다
  const data = readFileSync(filePath, 'utf-8');
  const configPath = path.resolve(filePath);
  const errorMsg = '설정 파일을 찾을 수 없습니다';
  console.log(configPath);
  return { data, error: errorMsg };
}

/* 모듈 내보내기:
   설정 로더를 외부에서 사용할 수 있도록 합니다.
*/
export { loadConfig };
