/*********************************************************************************************************************
 * CH32V307VCT6 Opensourec Library 即（CH32V307VCT6 开源库）是一个基于官方 SDK 接口的第三方开源库
 * Copyright (c) 2022 SEEKFREE 逐飞科技
 *
 * 本文件是CH32V307VCT6 开源库的一部分
 *
 * CH32V307VCT6 开源库 是免费软件
 * 您可以根据自由软件基金会发布的 GPL（GNU General Public License，即 GNU通用公共许可证）的条款
 * 即 GPL 的第3版（即 GPL3.0）或（您选择的）任何后来的版本，重新发布和/或修改它
 *
 * 本开源库的发布是希望它能发挥作用，但并未对其作任何的保证
 * 甚至没有隐含的适销性或适合特定用途的保证
 * 更多细节请参见 GPL
 *
 * 您应该在收到本开源库的同时收到一份 GPL 的副本
 * 如果没有，请参阅<https://www.gnu.org/licenses/>
 *
 * 额外注明：
 * 本开源库使用 GPL3.0 开源许可证协议 以上许可申明为译文版本
 * 许可申明英文版在 libraries/doc 文件夹下的 GPL3_permission_statement.txt 文件中
 * 许可证副本在 libraries 文件夹下 即该文件夹下的 LICENSE 文件
 * 欢迎各位使用并传播本程序 但修改内容时必须保留逐飞科技的版权声明（即本声明）
 *
 * 文件名称          isr
 * 公司名称          成都逐飞科技有限公司
 * 版本信息          查看 libraries/doc 文件夹内 version 文件 版本说明
 * 开发环境          MounRiver Studio V1.8.1
 * 适用平台          CH32V307VCT6
 * 店铺链接          https://seekfree.taobao.com/
 *
 * 修改记录
 * 日期                                      作者                             备注
 * 2022-09-15        大W            first version
 ********************************************************************************************************************/

/*
 * 中断功能节点总览（控制逻辑不在中断里，在主循环 step 4）：
 * DVP_IRQHandler    ：摄像头帧完成中断，置 mt9v03x_finish_flag（主循环 step 1 入口）
 * UART5_IRQHandler  ：摄像头串口接收
 * USART3_IRQHandler ：调试串口接收
 * UART7/UART8       ：无线模块 / GNSS 串口接收（预留）
 * TIM6_IRQHandler   ：10ms 心跳 g_tick_ms += 10，供主循环无帧超时停车判定
 * HardFault_Handler ：四路电机清零后死循环，防死机跑车
 * 其余 TIM/EXTI     ：空骨架，仅清中断标志
 */
#include "zf_common_headfile.h"
#define motor1_in1 TIM4_PWM_MAP1_CH1_D12
#define motor1_in2 TIM4_PWM_MAP1_CH2_D13
#define motor2_in1 TIM1_PWM_MAP3_CH2_E11
#define motor2_in2 TIM1_PWM_MAP3_CH1_E9

extern volatile uint32_t g_tick_ms;

void NMI_Handler (void) __attribute__ ((interrupt()));
void HardFault_Handler (void) __attribute__ ((interrupt()));

void USART1_IRQHandler (void) __attribute__ ((interrupt()));
void USART2_IRQHandler (void) __attribute__ ((interrupt()));
void USART3_IRQHandler (void) __attribute__ ((interrupt()));
void UART4_IRQHandler (void) __attribute__ ((interrupt()));
void UART5_IRQHandler (void) __attribute__ ((interrupt()));
void UART6_IRQHandler (void) __attribute__ ((interrupt()));
void UART7_IRQHandler (void) __attribute__ ((interrupt()));
void UART8_IRQHandler (void) __attribute__ ((interrupt()));
void DVP_IRQHandler (void) __attribute__ ((interrupt()));
void TIM1_UP_IRQHandler (void) __attribute__ ((interrupt()));
void TIM2_IRQHandler (void) __attribute__ ((interrupt()));
void TIM3_IRQHandler (void) __attribute__ ((interrupt()));
void TIM4_IRQHandler (void) __attribute__ ((interrupt()));
void TIM5_IRQHandler (void) __attribute__ ((interrupt()));
void TIM6_IRQHandler (void) __attribute__ ((interrupt()));
void TIM7_IRQHandler (void) __attribute__ ((interrupt()));
void TIM8_UP_IRQHandler (void) __attribute__ ((interrupt()));
void TIM9_UP_IRQHandler (void) __attribute__ ((interrupt()));
void TIM10_UP_IRQHandler (void) __attribute__ ((interrupt()));

void EXTI0_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI1_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI2_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI3_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI4_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI9_5_IRQHandler (void) __attribute__ ((interrupt()));
void EXTI15_10_IRQHandler (void) __attribute__ ((interrupt()));

void USART1_IRQHandler (void) {
    if (USART_GetITStatus (USART1, USART_IT_RXNE) != RESET) {

        USART_ClearITPendingBit (USART1, USART_IT_RXNE);
    }
}

void USART2_IRQHandler (void) {
    if (USART_GetITStatus (USART2, USART_IT_RXNE) != RESET) {


        USART_ClearITPendingBit (USART2, USART_IT_RXNE);
    }
}

void USART3_IRQHandler (void) {
    if (USART_GetITStatus (USART3, USART_IT_RXNE) != RESET) {
#if DEBUG_UART_USE_INTERRUPT        // 如果开启 debug 串口中断
        debug_interrupr_handler();  // 调用 debug 串口接收处理函数 数据会被 debug 环形缓冲区读取
#endif                              // 如果修改了 DEBUG_UART_INDEX 那这段代码需要放到对应的串口中断去
        USART_ClearITPendingBit (USART3, USART_IT_RXNE);
    }
}

void UART4_IRQHandler (void) {
    if (USART_GetITStatus (UART4, USART_IT_RXNE) != RESET) {

        USART_ClearITPendingBit (UART4, USART_IT_RXNE);
    }
}

void UART5_IRQHandler (void) {
    if (USART_GetITStatus (UART5, USART_IT_RXNE) != RESET) {
        camera_uart_handler();
        USART_ClearITPendingBit (UART5, USART_IT_RXNE);
    }
}

void UART6_IRQHandler (void) {
    if (USART_GetITStatus (UART6, USART_IT_RXNE) != RESET) {

        USART_ClearITPendingBit (UART6, USART_IT_RXNE);
    }
}

void UART7_IRQHandler (void) {
    if (USART_GetITStatus (UART7, USART_IT_RXNE) != RESET) {
        wireless_module_uart_handler();
        USART_ClearITPendingBit (UART7, USART_IT_RXNE);
    }
}

void UART8_IRQHandler (void) {
    if (USART_GetITStatus (UART8, USART_IT_RXNE) != RESET) {
        gnss_uart_callback();
        USART_ClearITPendingBit (UART8, USART_IT_RXNE);
    }
}

void DVP_IRQHandler (void) {
    if (DVP->IFR & RB_DVP_IF_FRM_DONE) {
        camera_dvp_handler();
        DVP->IFR &= ~RB_DVP_IF_FRM_DONE;
    }
}

void EXTI0_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line0)) {
        EXTI_ClearITPendingBit (EXTI_Line0);
    }
}

void EXTI1_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line1)) {
        EXTI_ClearITPendingBit (EXTI_Line1);
    }
}

void EXTI2_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line2)) {
        EXTI_ClearITPendingBit (EXTI_Line2);
    }
}

void EXTI3_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line3)) {
        EXTI_ClearITPendingBit (EXTI_Line3);
    }
}

void EXTI4_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line4)) {
        EXTI_ClearITPendingBit (EXTI_Line4);
    }
}

void EXTI9_5_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line5)) {
        EXTI_ClearITPendingBit (EXTI_Line5);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line6)) {
        EXTI_ClearITPendingBit (EXTI_Line6);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line7)) {
        EXTI_ClearITPendingBit (EXTI_Line7);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line8)) {
        EXTI_ClearITPendingBit (EXTI_Line8);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line9)) {
        EXTI_ClearITPendingBit (EXTI_Line9);
    }
}

void EXTI15_10_IRQHandler (void) {
    if (SET == EXTI_GetITStatus (EXTI_Line10)) {

        // 此处编写用户代码 (A10/B10..E10) 引脚触发

        // 此处编写用户代码 (A10/B10..E10) 引脚触发

        EXTI_ClearITPendingBit (EXTI_Line10);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line11)) {
        EXTI_ClearITPendingBit (EXTI_Line11);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line12)) {
        EXTI_ClearITPendingBit (EXTI_Line12);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line13)) {
        // -----------------* ToF INT 更新中断 预置中断处理函数 *-----------------
        tof_module_exti_handler();
        // -----------------* ToF INT 更新中断 预置中断处理函数 *-----------------
        // 此处编写用户代码 (A13/B13..E13) 引脚触发

        // 此处编写用户代码 (A13/B13..E13) 引脚触发

        EXTI_ClearITPendingBit (EXTI_Line13);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line14)) {
        // -----------------* DM1XA 光信号 预置中断处理函数 *-----------------
        dm1xa_light_callback();
        // -----------------* DM1XA 光信号 预置中断处理函数 *-----------------
        EXTI_ClearITPendingBit (EXTI_Line14);
    }
    if (SET == EXTI_GetITStatus (EXTI_Line15)) {
        // -----------------* DM1XA 声/反馈信号 预置中断处理函数 *-----------------
        dm1xa_sound_callback();
        // -----------------* DM1XA 声/反馈信号 预置中断处理函数 *-----------------
        EXTI_ClearITPendingBit (EXTI_Line15);
    }
}

void TIM1_UP_IRQHandler (void) {
    if (TIM_GetITStatus (TIM1, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM1, TIM_IT_Update);
    }
}

void TIM2_IRQHandler (void) {
    if (TIM_GetITStatus (TIM2, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM2, TIM_IT_Update);
    }
}

void TIM3_IRQHandler (void) {
    if (TIM_GetITStatus (TIM3, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM3, TIM_IT_Update);
    }
}

void TIM4_IRQHandler (void) {
    if (TIM_GetITStatus (TIM4, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM4, TIM_IT_Update);
    }
}

void TIM5_IRQHandler (void) {
    if (TIM_GetITStatus (TIM5, TIM_IT_Update) != RESET) {


        TIM_ClearITPendingBit (TIM5, TIM_IT_Update);
    }
}

// 心跳节点：只累加毫秒刻度，不做控制
void TIM6_IRQHandler (void) {
    if (TIM_GetITStatus (TIM6, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM6, TIM_IT_Update);
        g_tick_ms += 10;
    }
}

void TIM7_IRQHandler (void) {
    if (TIM_GetITStatus (TIM7, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM7, TIM_IT_Update);
    }
}

void TIM8_UP_IRQHandler (void) {
    if (TIM_GetITStatus (TIM8, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM8, TIM_IT_Update);
    }
}

void TIM9_UP_IRQHandler (void) {
    if (TIM_GetITStatus (TIM9, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM9, TIM_IT_Update);
    }
}

void TIM10_UP_IRQHandler (void) {
    if (TIM_GetITStatus (TIM10, TIM_IT_Update) != RESET) {
        TIM_ClearITPendingBit (TIM10, TIM_IT_Update);
    }
}

void NMI_Handler (void) {
}

// 安全节点：先停车再死循环
void HardFault_Handler (void) {
    pwm_set_duty (motor1_in1, 0);
    pwm_set_duty (motor1_in2, 0);
    pwm_set_duty (motor2_in1, 0);
    pwm_set_duty (motor2_in2, 0);
    while (1) {
    }
}
