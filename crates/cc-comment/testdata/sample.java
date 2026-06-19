package com.example.sample;

import java.util.List;
import java.util.ArrayList;
import com.example.model.User;

/**
 * 사용자 서비스 클래스입니다.
 * 사용자 데이터를 관리하고 비즈니스 로직을 처리합니다.
 */
public class UserService {

    // 사용자 목록을 내부적으로 관리합니다
    private List<User> users = new ArrayList<>();

    /**
     * 새 사용자를 추가합니다.
     * @param user 추가할 사용자 객체
     */
    public void addUser(User user) {
        // 입력 유효성 검사 후 추가
        if (user == null) {
            throw new IllegalArgumentException("사용자 객체가 null입니다");
        }
        users.add(user);
    }

    /* 내부 헬퍼:
       목록 크기를 반환합니다.
    */
    public int count() {
        return users.size();
    }
}
