#include "menu.h"
#include <stdio.h>
#include "zf_common_headfile.h"
#include "zf_device_tft180.h"
#include "flash.h"
#include "imu.h"
// ȫ�ֱ�������
uint8_t menu_level = 0;
uint8_t current_line = 0;
#define up B14     // �ϼ�
#define down B12   // �¼�
#define left C5    // ���
#define right E15  // �Ҽ�
#define mid A2     // �м�
uint8_t key_release = 1;

// ����ʵ��
void menu_init (void) {
    flash_init();
    tft180_init();
    tft180_clear();
    imu_init();
    gpio_init (up, GPI, 1, GPI_PULL_UP);
    gpio_init (left, GPI, 1, GPI_PULL_UP);
    gpio_init (right, GPI, 1, GPI_PULL_UP);
    gpio_init (down, GPI, 1, GPI_PULL_UP);
    gpio_init (mid, GPI, 1, GPI_PULL_UP);

    // ��ʼ������
}

void menu_task (void) {
    // �������
  imu_task();

    // ����
    if (menu_level == 0)                                                       // ���˵�
    {
        if (gpio_get_level (up) == 0 && current_line > 0 && key_release == 1)  // �϶�
        {
            current_line--;
            while (gpio_get_level (up) == 0) {
                system_delay_ms (20);
            }
        }
        if (gpio_get_level (down) == 0 && current_line < 2)  // �¶�
        {
            current_line++;
            while (gpio_get_level (down) == 0) {
                system_delay_ms (20);
            }
        }
        if (gpio_get_level (mid) == 0 && current_line == 0)  // ȷ������parameter
        {
            menu_level = 1;
            current_line = 0;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }
        if (gpio_get_level (mid) == 0 && current_line == 1)  // ȷ������functiong
        {
            menu_level = 2;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }
        if (gpio_get_level (mid) == 0 && current_line == 2)  // ����gyroscope��һҳ
        {
            menu_level = 3;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            
            }
        }
    } else if (menu_level == 1)                            // parameterҳ��
    {
        if (gpio_get_level (up) == 0 && current_line > 0)  // �϶�
        {
            while (gpio_get_level (up) == 0) {
                system_delay_ms (20);
            }
            current_line--;
        }
        if (gpio_get_level (down) == 0 && current_line < 2)  // �¶�
        {
            current_line++;
            while (gpio_get_level (down) == 0) {
                system_delay_ms (20);
            }
        }
        if (current_line == 0)               // ������
        {
            if (gpio_get_level (left) == 0)  // ��
            {
                my_int -= 10;
                while (gpio_get_level (left) == 0) {
                    system_delay_ms (20);
                }
            }
            if (gpio_get_level (right) == 0)  // ��
            {
                my_int += 10;
                while (gpio_get_level (right) == 0) {
                    system_delay_ms (20);
                }
            }
        }
        if (current_line == 1)               // ��������
        {
            if (gpio_get_level (left) == 0)  // ��
            {
                my_float -= 0.01;
                while (gpio_get_level (left) == 0) {
                    system_delay_ms (20);
                }
            }
            if (gpio_get_level (right) == 0)  // ��
            {
                my_float += 0.01;
                while (gpio_get_level (right) == 0) {
                    system_delay_ms (20);
                }
            }
        }
        if (gpio_get_level (mid) == 0 && current_line == 2)  // ����
        {
            menu_level = 0;
            current_line = 0;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }
        if (gpio_get_level (mid) == 0 && current_line != 2)  // ��������
        {
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }

            flash_task();

            tft180_clear();
        }

    } else if (menu_level == 2)  // function����
    {
        if (gpio_get_level (mid) == 0) {
            menu_level = 0;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }
    } else if (menu_level == 3)  // ��gyroscope��һҳ
    {
        if (gpio_get_level (mid) == 0) {
            menu_level = 4;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }

    } else if (menu_level == 4)  // ��gyroscope�ڶ�ҳ
    {
        if (gpio_get_level (mid) == 0) {
            menu_level = 0;
            current_line = 0;
            while (gpio_get_level (mid) == 0) {
                system_delay_ms (20);
            }
        }
    }
    // ����Ļ
    tft180_clear();
    if (menu_level == 0)  // ���˵�ҳ��
    {                     // ����
        tft180_show_string (20, 40, "Parameters");
        tft180_show_string (20, 70, "Functions");
        tft180_show_string (20, 100, "Gyroscope");
        // ���
        if (current_line == 0) {
            tft180_show_string (10, 40, ">");
        }
        if (current_line == 1) {
            tft180_show_string (10, 70, ">");
        }
        if (current_line == 2) {
            tft180_show_string (10, 100, ">");
        }
    } else if (menu_level == 1)  // parameter�˵�ҳ��
    {
  
        tft180_show_string (10, 40, "I :");
        tft180_show_int (20, 40, my_int, 4);  

        // ��ʾ float
        tft180_show_string (10, 70, "F:");
        tft180_show_float (20, 70, my_float, 1, 2);  
       //save
        tft180_show_string (10, 100, "save");
        // ���
        if (current_line == 0) {
            tft180_show_string (5, 40, ">");
        }
        if (current_line == 1) {
            tft180_show_string (5, 70, ">");
        }
        if (current_line == 2) {
            tft180_show_string (5, 100, ">");
        }
    } else if (menu_level == 2)  // function����
    {
        tft180_full (RGB565_YELLOW);
        tft180_draw_point (10, 10, RGB565_BLACK);
        tft180_draw_line (20, 20, 70, 70, RGB565_BLACK);
    } else if (menu_level == 3) {
 
        tft180_show_string (0, 0, "acc_x:");
        tft180_show_int (70, 0, imu660ra_acc_x, 5);
        tft180_show_string (0, 16, "acc_y:");
        tft180_show_int (70, 16, imu660ra_acc_y, 5);
        tft180_show_string (0, 32, "acc_z:");
        tft180_show_int (70, 32, imu660ra_acc_z, 5);
        tft180_show_string (0, 48, "gyro_x:");
        tft180_show_int (70, 48, imu660ra_gyro_x, 5);
        tft180_show_string (0, 64, "gyro_y:");
        tft180_show_int (70, 64, imu660ra_gyro_y, 5);
        tft180_show_string (0, 80, "gyro_z:");
        tft180_show_int (70, 80, imu660ra_gyro_z, 5);

    } else if (menu_level == 4) {

        tft180_show_string (0, 10, "roll:");
        tft180_show_float (30, 10, roll, 3, 2);  // ����3λ + С��2λ
        tft180_show_string (0, 40, "pitch:");
        tft180_show_float (30, 40, pitch, 3, 2);
        tft180_show_string (0, 70, "yaw:");
        tft180_show_float (30, 70, yaw, 3, 2);
    }
}