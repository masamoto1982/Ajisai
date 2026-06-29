from fractions import Fraction
import random, math

def rcf_terms(fr, cap=200):
    """Regular continued fraction partial quotients (floor)."""
    n, d = fr.numerator, fr.denominator
    out=[]
    for _ in range(cap):
        if d==0: break
        a = n//d            # floor division
        out.append(a)
        n, d = d, n - a*d
        if d==0: break
    return out

def nicf_terms(fr, cap=200):
    """Nearest-integer continued fraction, round-half-down tie-break
       (remainder in (-1/2, 1/2]) as in Ajisai SPEC 4.2.5."""
    n, d = fr.numerator, fr.denominator
    out=[]
    for _ in range(cap):
        if d==0: break
        # nearest integer to n/d with den>0, round half DOWN:
        if d<0: n,d=-n,-d
        a = -((-(2*n - d))//(2*d))   # ceil((2n-d)/(2d))
        out.append(a)
        n, d = d, n - a*d           # reciprocal of (value - a)
        if d==0: break
    return out

def first_divergence(t1,t2):
    """index of first differing term, treating end-of-stream as a difference."""
    i=0
    while i<len(t1) and i<len(t2) and t1[i]==t2[i]:
        i+=1
    return i

random.seed(1)
rcf_wins=nicf_wins=tie=0
rcf_total=nicf_total=0
N=20000
for _ in range(N):
    # two distinct nearby rationals
    a=Fraction(random.randint(-10000,10000), random.randint(1,5000))
    b=a+Fraction(1, random.randint(2, 10**6))
    r=first_divergence(rcf_terms(a),rcf_terms(b))
    s=first_divergence(nicf_terms(a),nicf_terms(b))
    rcf_total+=r; nicf_total+=s
    if s<r: nicf_wins+=1
    elif s>r: rcf_wins+=1
    else: tie+=1

print(f"pairs tested: {N}")
print(f"NICF diverges EARLIER (fewer terms to decide): {nicf_wins} ({100*nicf_wins/N:.1f}%)")
print(f"RCF diverges earlier:                          {rcf_wins} ({100*rcf_wins/N:.1f}%)")
print(f"tie:                                           {tie} ({100*tie/N:.1f}%)")
print(f"mean terms to decide  RCF={rcf_total/N:.3f}  NICF={nicf_total/N:.3f}  speedup x{rcf_total/nicf_total:.3f}")

# Levy-type growth: denominator of nth convergent ~ exp(K n)
# Empirically estimate K (Levy constant ~ 1.18657 = pi^2/(12 ln2)) for RCF
def conv_denoms(terms):
    k0,k1=1,0
    out=[]
    for a in terms:
        k0,k1 = a*k0+k1, k0
        out.append(k0)
    return out
import statistics
ks=[]
for _ in range(3000):
    x=Fraction(random.randint(1,10**12), random.randint(1,10**12))
    t=rcf_terms(x,cap=40)
    dens=conv_denoms(t)
    if len(dens)>=20 and dens[19]>0:
        ks.append(math.log(dens[19])/20)
print(f"\nempirical RCF Levy constant K ~ {statistics.mean(ks):.5f}  (theory pi^2/(12 ln2)={math.pi**2/(12*math.log(2)):.5f})")

print("\n--- hunting counterexamples where RCF decides in strictly fewer terms ---")
random.seed(7)
found=0
for _ in range(200000):
    a=Fraction(random.randint(-50,50), random.randint(1,40))
    b=a+Fraction(1, random.randint(2, 2000))
    rt,nt=rcf_terms(a),rcf_terms(b)
    rt2,nt2=nicf_terms(a),nicf_terms(b)
    r=first_divergence(rt,nt)
    s=first_divergence(rt2,nt2)
    if s>r:
        found+=1
        print(f"a={a}  b={b}")
        print(f"   RCF(a)={rt}  RCF(b)={nt}   -> diverge at {r}")
        print(f"  NICF(a)={rt2}  NICF(b)={nt2}  -> diverge at {s}")
        if found>=4: break
