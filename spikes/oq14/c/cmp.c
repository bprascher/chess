// Comparison only: fsync vs fcntl(F_FULLFSYNC) timing, and renamex_np(RENAME_EXCL).
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <errno.h>
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec*1e3+t.tv_nsec/1e6;}
int main(int argc,char**argv){
  int fd=open(argv[1],O_RDWR|O_CREAT,0600); int n=50; char b[64]; memset(b,1,64);
  double t=now(); for(int i=0;i<n;i++){write(fd,b,64);fsync(fd);} printf("C fsync mean %.3f ms\n",(now()-t)/n);
  t=now(); for(int i=0;i<n;i++){write(fd,b,64);fcntl(fd,F_FULLFSYNC);} printf("C F_FULLFSYNC mean %.3f ms\n",(now()-t)/n);
  return 0;}
