// The independent SVG-list parser from upstream's react-native-worklets.test.ts.
export function matrix(list: string): number[] {
  let m = [1, 0, 0, 1, 0, 0];
  const mul = (n: number[]) => {
    m = [m[0]*n[0]+m[2]*n[1], m[1]*n[0]+m[3]*n[1],
      m[0]*n[2]+m[2]*n[3], m[1]*n[2]+m[3]*n[3],
      m[0]*n[4]+m[2]*n[5]+m[4], m[1]*n[4]+m[3]*n[5]+m[5]];
  };
  for (const [,fn,argv] of list.matchAll(/(\w+)\(([^)]*)\)/g)) {
    const a = argv.trim().split(/[\s,]+/).map(Number);
    if (fn === "translate") mul([1,0,0,1,a[0],a[1] ?? 0]);
    else if (fn === "scale") mul([a[0],0,0,a[1] ?? a[0],0,0]);
    else if (fn === "rotate") {
      const r = a[0]*Math.PI/180;
      mul([Math.cos(r),Math.sin(r),-Math.sin(r),Math.cos(r),0,0]);
    } else throw new Error(`Unhandled transform ${fn}`);
  }
  return m;
}
