"""Replace the client's payment buttons with authenticated free local product claims."""
from __future__ import annotations

CLAIM_METHOD = r'''
      private function claimFreeProduct(productId:String) : void
      {
         var owner:* = this.main;
         owner.loading(true);
         var requestId:String = String(new Date().time) + "-" + String(Math.random()).replace(".", "");
         owner.amf_manager.service("Billing.claimProduct",[Storage.Character.char_id,Storage.Character.sessionkey,productId,requestId],function(response:Object):void
         {
            owner.loading(false);
            if(response && response.status == 1)
            {
               Storage.Character.addRewards(response.rewards);
               Storage.Character.account_tokens = response.tokens;
               Storage.Character.account_type = response.account_type;
               Storage.Character.emblem_duration = -1;
               if(owner.HUD) owner.HUD.setBasicData();
               owner.showMessage(response.result);
            }
            else
            {
               owner.showMessage(response && response.result ? response.result : "Unable to claim free product.");
            }
         });
      }
'''

def patch_claim(source: str, old: str, new: str) -> str:
    if 'private function claimFreeProduct(' in source:
        return source
    if old not in source:
        raise ValueError('Expected payment action not found')
    source=source.replace(old,new)
    end=source.rfind('   }')
    if end<0:
        raise ValueError('Class closing delimiter not found')
    return source[:end]+CLAIM_METHOD+'\n'+source[end:]

def purchase(source: str) -> str:
    return patch_claim(source,'this.main.payment.purchaseProduct(param1.currentTarget.metaData.packageId);','this.claimFreeProduct(param1.currentTarget.metaData.packageId);')

def recharge(source: str) -> str:
    result=patch_claim(source,'navigateToURL(new URLRequest("https://ninjasage.id/merchants"));','this.claimFreeProduct(param1.currentTarget.metaData.packageId);')
    result=result.replace('this.btn_BonusRecharge.addEventListener(MouseEvent.CLICK,this.buyPack);','this.btn_BonusRecharge.visible = false;')
    # Recharge-only coupons are obsolete when their products are directly free.
    return result.replace('this.btn_RedeemTicket.addEventListener(MouseEvent.CLICK,this.buyPack);','this.btn_RedeemTicket.visible = false;')

def packages(source: str) -> str:
    return patch_claim(source,'navigateToURL(new URLRequest("https://ninjasage.id/merchants"));','this.claimFreeProduct("id.ninjasage.elementalars." + String(this.selectedPackage + 1));')

def headquarter(source: str) -> str:
    return source.replace('navigateToURL(new URLRequest("https://ninjasage.id/merchants"));','this.main.loadPanel("Panels.Recharge");')

PATCHES={
 'Panels.Recharge':recharge,
 'Popups.EmblemUpgrade':purchase,
 'Panels.TrialEmblem':purchase,
 'id.ninjasage.features.ChuninPackages':purchase,
 'id.ninjasage.features.ElementalArsPackage':packages,
 'id.ninjasage.features.MekkorvathPackage':packages,
 'id.ninjasage.features.Headquarter':headquarter,
}
VERIFICATIONS={name:['Panels.Recharge'] if name.endswith('Headquarter') else ['Billing.claimProduct','claimFreeProduct'] for name in PATCHES}
