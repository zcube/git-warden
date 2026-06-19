import { useState, useEffect } from 'react';
import type { FC } from 'react';
import axios from 'axios';
import 'reflect-metadata';

// 사용자 정보를 나타내는 인터페이스
interface User {
  id: number;
  name: string;
}

/**
 * 사용자 목록을 불러오는 훅입니다.
 * 비동기 데이터 로딩을 처리합니다.
 */
function useUsers(): User[] {
  const [users, setUsers] = useState<User[]>([]);

  useEffect(() => {
    // API에서 사용자 데이터를 가져옵니다
    axios.get('/api/users').then(res => {
      setUsers(res.data);
    });
  }, []);

  const label = `총 사용자 수: ${users.length}명`; // 템플릿 리터럴
  console.log(label);

  return users;
}

export default useUsers;
