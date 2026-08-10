# Is g(Y) = optimal value of the capacitated transportation LP submodular in the open-node set Y?
# g(Y) = max sum_z sum_{f in Y} (m - c_fz) lam_z x_fz   s.t. sum_f x_fz <= 1,  sum_z lam_z x_fz <= mu_f
import itertools, random
from scipy.optimize import linprog

def g(Y, prof, lam, mu, nF, nZ):
    if not Y: return 0.0
    Y=sorted(Y); idx={f:i for i,f in enumerate(Y)}
    nv=len(Y)*nZ
    c=[-prof[f][z]*lam[z] for f in Y for z in range(nZ)]
    A=[];b=[]
    for z in range(nZ):                      # each zone served at most once
        A.append([1.0 if zz==z else 0.0 for f in Y for zz in range(nZ)]); b.append(1.0)
    for f in Y:                              # node throughput
        A.append([lam[zz] if ff==f else 0.0 for ff in Y for zz in range(nZ)]); b.append(mu[f])
    r=linprog(c,A_ub=A,b_ub=b,bounds=[(0,1)]*nv,method="highs")
    return -r.fun if r.success else 0.0

def violation(prof,lam,mu,nF,nZ,tol=1e-7):
    F=list(range(nF)); worst=0; wit=None
    for S in itertools.chain.from_iterable(itertools.combinations(F,k) for k in range(nF)):
        S=set(S)
        for T in itertools.chain.from_iterable(itertools.combinations(F,k) for k in range(nF)):
            T=set(T)
            if not S<T: continue
            for f in F:
                if f in T: continue
                d=(g(S|{f},prof,lam,mu,nF,nZ)-g(S,prof,lam,mu,nF,nZ)) - \
                  (g(T|{f},prof,lam,mu,nF,nZ)-g(T,prof,lam,mu,nF,nZ))
                if d < -tol and -d>worst: worst=-d; wit=(sorted(S),sorted(T),f)
    return worst,wit

random.seed(7); found=None
for trial in range(300):
    nF,nZ=3,3
    prof=[[round(random.uniform(0,1),2) for _ in range(nZ)] for _ in range(nF)]
    lam=[round(random.uniform(.2,1.),2) for _ in range(nZ)]
    mu=[round(random.uniform(.2,1.5),2) for _ in range(nF)]
    w,wit=violation(prof,lam,mu,nF,nZ)
    if w>1e-6: found=(prof,lam,mu,w,wit); break
print("CAPACITATED (random search, 3x3, 300 trials):", "VIOLATION FOUND" if found else "no violation found")
if found: print(found)

# uncapacitated control: must be submodular
random.seed(7)
nF,nZ=3,3
prof=[[round(random.uniform(0,1),2) for _ in range(nZ)] for _ in range(nF)]
lam=[round(random.uniform(.2,1.),2) for _ in range(nZ)]
w,_=violation(prof,lam,[1e6]*nF,nF,nZ)
print("UNCAPACITATED control worst violation:",w)

print("\n--- harder adversarial search: 4 nodes, tight capacities, structured profits ---")
random.seed(11); found=None
for trial in range(400):
    nF,nZ=4,4
    # tight capacities force competition; profits clustered so ties/complementarity are likely
    prof=[[round(random.choice([0.1,0.2,0.5,0.9,1.0]),2) for _ in range(nZ)] for _ in range(nF)]
    lam=[round(random.choice([0.3,0.5,1.0]),2) for _ in range(nZ)]
    mu =[round(random.choice([0.3,0.5,0.6,1.0]),2) for _ in range(nF)]
    w,wit=violation(prof,lam,mu,nF,nZ)
    if w>1e-6: found=(prof,lam,mu,w,wit); break
print("VIOLATION FOUND" if found else "no violation found in 400 adversarial 4x4 instances")
if found: print(found)

print("\n--- integral (unsplittable) assignment: x in {0,1} ---")
def g_int(Y,prof,lam,mu,nF,nZ):
    if not Y: return 0.0
    best=0.0
    for assign in itertools.product(list(Y)+[None],repeat=nZ):
        load={f:0.0 for f in Y}; val=0.0; ok=True
        for z,f in enumerate(assign):
            if f is None: continue
            load[f]+=lam[z]; val+=prof[f][z]*lam[z]
            if load[f]>mu[f]+1e-9: ok=False;break
        if ok: best=max(best,val)
    return best
def violation_int(prof,lam,mu,nF,nZ,tol=1e-7):
    F=list(range(nF)); worst=0; wit=None
    for k in range(nF):
        for S in itertools.combinations(F,k):
            S=set(S)
            for k2 in range(k+1,nF):
                for T in itertools.combinations(F,k2):
                    T=set(T)
                    if not S<T: continue
                    for f in F:
                        if f in T: continue
                        d=(g_int(S|{f},prof,lam,mu,nF,nZ)-g_int(S,prof,lam,mu,nF,nZ))-\
                          (g_int(T|{f},prof,lam,mu,nF,nZ)-g_int(T,prof,lam,mu,nF,nZ))
                        if d<-tol and -d>worst: worst=-d; wit=(sorted(S),sorted(T),f,prof,lam,mu)
    return worst,wit
random.seed(3); f2=None
for trial in range(4000):
    nF,nZ=3,3
    prof=[[random.choice([0.2,0.6,1.0]) for _ in range(nZ)] for _ in range(nF)]
    lam=[random.choice([0.4,0.6,1.0]) for _ in range(nZ)]
    mu =[random.choice([0.4,0.6,1.0]) for _ in range(nF)]
    w,wit=violation_int(prof,lam,mu,nF,nZ)
    if w>1e-6: f2=(w,wit); break
print("VIOLATION FOUND" if f2 else "no violation in 4000 integral instances")
if f2: print("worst gap",round(f2[0],4)); print("S,T,f =",f2[1][:3]); print("prof",f2[1][3],"lam",f2[1][4],"mu",f2[1][5])
