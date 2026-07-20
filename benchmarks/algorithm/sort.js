function min3(a, b, c) {
    if (a <= b) { if (a <= c) { return a } else { return c } }
    else { if (b <= c) { return b } else { return c } }
}

function max3(a, b, c) {
    if (a >= b) { if (a >= c) { return a } else { return c } }
    else { if (b >= c) { return b } else { return c } }
}

function mid3(a, b, c) {
    return a + b + c - min3(a, b, c) - max3(a, b, c);
}

const a = 9, b = 3, c = 7;
const start = process.hrtime.bigint();
const lo = min3(a, b, c);
const mi = mid3(a, b, c);
const hi = max3(a, b, c);
const elapsed = Number(process.hrtime.bigint() - start);
console.log(`${lo} ${mi} ${hi} (${elapsed}ns)`);
