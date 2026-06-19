package com.example.sample

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import com.example.model.Item

// 아이템 저장소 인터페이스
interface ItemRepository {
    // 모든 아이템을 반환합니다
    suspend fun findAll(): List<Item>
}

/**
 * 데이터베이스 기반 아이템 저장소 구현체.
 * 코루틴을 활용하여 비동기로 동작합니다.
 */
class DbItemRepository : ItemRepository {

    override suspend fun findAll(): List<Item> = withContext(Dispatchers.IO) {
        /* 데이터베이스 쿼리 실행:
           IO 디스패처에서 실행하여 메인 스레드를 차단하지 않습니다.
        */
        val errorMessage = "아이템을 찾을 수 없습니다"
        println(errorMessage)
        emptyList()
    }
}
