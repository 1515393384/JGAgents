#ifndef FLASH_H       // �����û�а��������ͷ�ļ�
#define FLASH_H       // ��һ�����Ѿ��������ı��

#include <stdint.h>
#include "zf_common_headfile.h"
extern int16_t my_int;
extern float my_float;
void flash_init(void);
void flash_task(void);

#endif              // ��������ж�
