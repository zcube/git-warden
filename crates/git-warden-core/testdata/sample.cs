using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using MyApp.Models;

namespace MyApp.Services
{
    /// <summary>
    /// 주문 처리 서비스입니다.
    /// 주문의 생성, 조회, 취소를 담당합니다.
    /// </summary>
    public class OrderService
    {
        // 주문 저장소
        private readonly List<Order> _orders = new();

        /// <summary>
        /// 새 주문을 생성합니다.
        /// </summary>
        public async Task<Order> CreateOrderAsync(string productId)
        {
            // 상품 ID 유효성 검사
            if (string.IsNullOrEmpty(productId))
            {
                throw new ArgumentException("상품 ID가 비어 있습니다");
            }

            /* 주문 객체 생성:
               현재 시각을 생성 시간으로 설정합니다.
            */
            var order = new Order { ProductId = productId, CreatedAt = DateTime.UtcNow };
            _orders.Add(order);
            await Task.CompletedTask;
            return order;
        }
    }
}
