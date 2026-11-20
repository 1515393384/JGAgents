#include "flash.h"
#include "zf_common_headfile.h"

// // ȫ�ֱ�������
int16 my_int = 100;
float my_float = 0.98;

// // ����ʵ��
void flash_init (void) {
    flash_read_page_to_buffer (62, 1);
    my_int = flash_union_buffer[0].int16_type;
    my_float = flash_union_buffer[1].float_type;
    if (my_int < -500 || my_int > 500) {
        my_int = 100;
    }
    if (my_float < -100.0 || my_float > 100.0) {
        my_float = 0.98;
        // ��ʼ������
    }
}

void flash_task (void) {
    __disable_irq();
    flash_check (62, 1);
    flash_erase_sector (62, 1);
    flash_union_buffer[0].int16_type = my_int;
    flash_union_buffer[1].float_type = my_float;
    flash_write_page_from_buffer (62, 1);
    __enable_irq();
}

// �������
