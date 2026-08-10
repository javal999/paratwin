import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import rcParams
rcParams.update({"font.family":"serif","font.size":8,"axes.linewidth":.6})
F,a,b=1.2,6.,120.
Psi=lambda n,r: np.sqrt(n/(1+(n-1)*r))
C=lambda n,r: F*n+a*Psi(n,r)+b*n**-0.5
n=np.linspace(1,40,4000)
fig,ax=plt.subplots(figsize=(3.4,2.5))
cols=["#1b3a5c","#2f6f9f","#5aa0c8","#9dc6de"]
for i,r in enumerate([0.0,0.1,0.3,0.6]):
    v=C(n,r); j=v.argmin()
    ax.plot(n,v,color=cols[i],lw=1.3,label=rf"$\rho={r}$")
    ax.plot(n[j],v[j],'o',color=cols[i],ms=3.5)
    ax.annotate(rf"$n^\star\!=\!{n[j]:.1f}$",(n[j],v[j]),textcoords="offset points",
                xytext=(4,-9),fontsize=6.2,color=cols[i])
ax.axvspan(0,4.5,color="#d94f4f",alpha=.10)
ax.text(1.15,150,"infeasible\n"+r"($n<n_{\min}$)",fontsize=6,color="#a33",va="top")
ax.axvline(4.5,color="#d94f4f",lw=.8,ls="--")
ax.set_xlabel("nodes per metro $n$"); ax.set_ylabel("cost rate $C(n)$ (index)")
ax.set_xlim(0,40); ax.set_ylim(40,170)
ax.legend(frameon=False,fontsize=6.5,loc="upper right",title=r"zone-demand correlation",title_fontsize=6.5)
ax.spines[['top','right']].set_visible(False)
fig.tight_layout(pad=.3); fig.savefig("costcurve.pdf")
print("ok")
