import numpy as np, itertools, math
from scipy.optimize import brentq
from scipy.stats import norm

print("="*70); print("M3: equicorrelated pooling  Psi(n,rho)=sqrt(n/(1+(n-1)rho))")
Psi=lambda n,r: np.sqrt(n/(1+(n-1)*r))
for r in (0,.1,.3,.5,.9,1.0):
    print(f" rho={r}: ",[round(Psi(n,r),3) for n in (1,4,9,16,25,100)])

print("\n Theorem 3': C(n)=F n + a Psi(n,rho) + b n^-0.5 ; F,a,b=1.2,6,120")
F,a,b=1.2,6.,120.
C=lambda n,r: F*n+a*Psi(n,r)+b*n**-0.5
for r in (0,.05,.1,.2,.3,.5,.7,.9):
    ns=np.linspace(.5,120,400000); vals=C(ns,r); i=vals.argmin()
    d=np.diff(np.sign(np.diff(vals)))          # sign changes of the derivative
    turns=int((d!=0).sum())
    print(f" rho={r:<4} n*={ns[i]:7.3f}  C*={vals[i]:7.3f}  interior turning points={turns} (1 => unimodal)")

print("\n"+"="*70); print("M5: wave batching. within-wave arrivals Poisson(a), a=lambda*Delta")
print(" index of dispersion I=E[X^2]/E[X]=1+a  => safety-staffing inflation sqrt(1+a)")
for name,lam_h in (("store node",10),("dark MFC",60),("flagship",150)):
    for D in (5,15,30):
        a_=lam_h/3600*D
        print(f"  {name:<10} lam={lam_h:3d}/h  Delta={D:2d}s  a={a_:5.3f}  inflation={math.sqrt(1+a_):.3f}")
print("\n staffing effect, beta_delta=1:")
for rho in (5,20,50):
    for a_ in (0,0.25,1.25,4):
        base=rho+1*math.sqrt(rho); corr=rho+1*math.sqrt(1+a_)*math.sqrt(rho)
        print(f"  rho={rho:3d} a={a_:4.2f}: c_v4={base:6.2f} c_v5={corr:6.2f} extra pickers={corr-base:5.2f}")

print("\n"+"="*70); print("M1: basket fill. beta* solves G(beta)=sum p_k beta^k = alpha_B")
def beta_star(pk,alpha):
    G=lambda x: sum(p*x**k for k,p in pk.items())
    return brentq(lambda x: G(x)-alpha, 1e-9, 1-1e-15)
for label,pk in (("mean 1.8 items",{1:0.45,2:0.35,3:0.15,4:0.05}),
                 ("mean 2.6 items",{1:0.25,2:0.30,3:0.25,4:0.15,5:0.05}),
                 ("mean 3.4 items",{1:0.10,2:0.25,3:0.30,4:0.20,5:0.15})):
    m=sum(k*p for k,p in pk.items()); bs=beta_star(pk,0.95)
    print(f" {label:<16} E[k]={m:.2f}  beta*={bs:.4f}  z_beta={norm.ppf(bs):.3f}  (v4 z at 0.95={norm.ppf(.95):.3f})")
    print(f"    -> order fill if beta=0.98: {sum(p*0.98**k for k,p in pk.items()):.4f}")

print("\n"+"="*70); print("M2: Boole split.  R(e1,e2)=v_(1-e2)*(tau-Troute-t_node^(1-e1))+,  e1+e2=1-alpha")
tau,Troute,alpha=60.,4.,0.95
# in-node time lognormal (report says lognormal), travel speed quantiles
mu,sig=math.log(9),0.35           # in-node ~ median 9 min
tq=lambda q: math.exp(mu+sig*norm.ppf(q))
v_mean=20.
vq=lambda q: v_mean*math.exp(-0.30*norm.ppf(q))   # slower speed at high quantile
best=None
for e1 in np.linspace(0.001,0.049,49):
    e2=0.05-e1
    budget=tau-Troute-tq(1-e1)
    if budget<=0: continue
    R=vq(1-e2)/60*budget
    if best is None or R>best[0]: best=(R,e1,e2,budget)
R4=v_mean/60*(tau-Troute-tq(alpha))
print(f" v4  R = {R4:.3f} km   (single alpha-quantile of in-node, mean speed)")
print(f" v5  R*= {best[0]:.3f} km at e1={best[1]:.4f}, e2={best[2]:.4f}, node budget={best[3]:.2f} min")
A,eta=180.,0.65
nmin=lambda R: math.ceil(A/(eta*math.pi*R*R))
print(f" n_min (A=180km2, eta=0.65):  v4={nmin(R4)}   v5={nmin(best[0])}   ratio={(R4/best[0])**2:.2f}x")
