#ifndef FLASH_H       // 如果还没有包含过这个头文件
#define FLASH_H       // 做一个“已经包含”的标记

#include <stdint.h>
#include "zf_common_headfile.h"
extern int16_t my_int;
extern float my_float;
void flash_init(void);
void flash_task(void);

#endif              // 结束这次判断
