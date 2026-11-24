#include <stdio.h>
#include <stdlib.h>
#include <sys/stdio.h>
#include <errno.h>
#include <string.h>
int main(int argc,char**argv){
  int r=renamex_np(argv[1],argv[2],RENAME_EXCL);
  printf("renamex_np RENAME_EXCL -> %d errno=%d (%s)\n",r,errno,r?strerror(errno):"ok");return 0;}
